//! UI world-space + screen-space per mostrare l'avanzamento dei cast/channeling.
//!
//! Driver unico: i messaggi replicati `SpellCastProgress` e `SpellCastEnded`.
//! Tutti i client (incluso quello del caster) vedono le barre sopra i caster.

use bevy::prelude::*;
use std::collections::HashMap;

use crate::game_state::{GameScreen, Screen};
use crate::network::client::ConnectedClient;
use crate::network::mode::has_client;
use crate::network::protocol::{NetworkEntityId, SpellCastEnded, SpellCastProgress};
use crate::ui::theme::UiTheme;
use lightyear::prelude::MessageReceiver;

/// Entry locale che rispecchia un `SpellCastProgress` ricevuto dal server.
#[derive(Debug, Clone)]
pub struct ObservedCast {
    pub spell_id: String,
    pub kind: u8,
    pub elapsed_seconds: f32,
    pub required_seconds: f32,
    /// Tempo (in secondi di client) dall'ultimo update ricevuto. Usato per
    /// estrapolare l'avanzamento tra un update e il successivo.
    pub since_update_seconds: f32,
    /// Tick di scadenza: se non riceviamo +update entro questo tempo, rimuoviamo.
    pub stale_after_seconds: f32,
}

/// Stato osservato dei cast in corso nel mondo.
#[derive(Resource, Default)]
pub struct ObservedCasts(pub HashMap<u64, ObservedCast>);

/// Componente marker per la barra world-space di un caster.
#[derive(Component)]
pub struct WorldCastBar {
    pub caster_network_id: u64,
}

/// Plugin hook (chiamato da `super::plugin`).
pub fn cast_bar_systems(app: &mut App) {
    app.init_resource::<ObservedCasts>();
    app.add_systems(
        Update,
        (read_cast_progress, read_cast_ended, sync_world_cast_bars)
            .chain()
            .run_if(has_client)
            .run_if(in_gameplay_or_paused),
    );
    app.add_systems(
        Update,
        cleanup_world_cast_bars
            .run_if(has_client)
            .run_if(not_in_gameplay_or_paused),
    );
}

fn in_gameplay_or_paused(screen: Res<GameScreen>) -> bool {
    matches!(screen.0, Screen::InGame | Screen::Paused)
}

fn not_in_gameplay_or_paused(screen: Res<GameScreen>) -> bool {
    !in_gameplay_or_paused(screen)
}

/// Lettore dei `SpellCastProgress` dal canale server→client (via receiver
/// attached al client) oppure da messaggi lightyear.
fn read_cast_progress(
    time: Res<Time>,
    mut observed: ResMut<ObservedCasts>,
    mut receivers: Query<&mut MessageReceiver<SpellCastProgress>, With<ConnectedClient>>,
) {
    let delta = time.delta_secs();
    // tick existing entries
    for entry in observed.0.values_mut() {
        entry.since_update_seconds += delta;
        entry.elapsed_seconds += delta;
    }
    // purge stale (> 1s senza update)
    observed
        .0
        .retain(|_, entry| entry.since_update_seconds < entry.stale_after_seconds);

    for mut receiver in receivers.iter_mut() {
        for message in receiver.receive() {
            observed.0.insert(
                message.caster_network_id,
                ObservedCast {
                    spell_id: message.spell_id.clone(),
                    kind: message.kind,
                    elapsed_seconds: message.elapsed_seconds,
                    required_seconds: message.required_seconds,
                    since_update_seconds: 0.0,
                    stale_after_seconds: 1.0,
                },
            );
        }
    }
}

fn read_cast_ended(
    mut observed: ResMut<ObservedCasts>,
    mut receivers: Query<&mut MessageReceiver<SpellCastEnded>, With<ConnectedClient>>,
) {
    for mut receiver in receivers.iter_mut() {
        for message in receiver.receive() {
            observed.0.remove(&message.caster_network_id);
        }
    }
}

/// Sincronizza le entità UI `WorldCastBar` con lo stato osservato: spawn,
/// despawn, fill width, label.
fn sync_world_cast_bars(
    mut commands: Commands,
    theme: Res<UiTheme>,
    observed: Res<ObservedCasts>,
    casters: Query<(Entity, &NetworkEntityId), Without<WorldCastBar>>,
    bar_owners: Query<(&WorldCastBar, Entity, &Children)>,
    mut bar_fill_query: Query<&mut Node, With<CastBarFill>>,
    mut label_query: Query<&mut Text, With<CastBarText>>,
) {
    // 1. Despawn bars whose caster is no longer observed.
    let observed_ids: Vec<u64> = observed.0.keys().copied().collect();
    let mut to_despawn: Vec<Entity> = Vec::new();
    for (bar, entity, _) in bar_owners.iter() {
        if !observed_ids.contains(&bar.caster_network_id) {
            to_despawn.push(entity);
        }
    }
    for entity in to_despawn {
        commands.entity(entity).despawn();
    }

    // 2. Spawn bars for newly-observed casters.
    let existing_ids: Vec<u64> = bar_owners
        .iter()
        .map(|(bar, _, _)| bar.caster_network_id)
        .collect();
    for (caster_entity, network_id) in casters.iter() {
        if !observed.0.contains_key(&network_id.0) {
            continue;
        }
        if existing_ids.contains(&network_id.0) {
            continue;
        }
        let bar_entity = spawn_world_bar(&mut commands, &theme, network_id.0);
        commands
            .entity(bar_entity)
            .set_parent_in_place(caster_entity);
    }

    // 3. Update fill width + label per ogni barra esistente.
    for (bar, _entity, children) in bar_owners.iter() {
        let Some(observed_cast) = observed.0.get(&bar.caster_network_id) else {
            continue;
        };
        let progress = if observed_cast.required_seconds > 0.0 {
            (observed_cast.elapsed_seconds / observed_cast.required_seconds).clamp(0.0, 1.0)
        } else {
            // Channeling aperto: barra pulsante al 50%.
            0.5 + (observed_cast.elapsed_seconds * 4.0).sin() * 0.2
        };
        let remaining = (observed_cast.required_seconds - observed_cast.elapsed_seconds).max(0.0);
        let label_text = if observed_cast.kind == 1 {
            format!("CH {:.1}s", observed_cast.elapsed_seconds)
        } else {
            format!("{} {:.1}s", observed_cast.spell_id, remaining)
        };

        for child in children.iter() {
            if let Ok(mut fill_node) = bar_fill_query.get_mut(child) {
                fill_node.width = Val::Percent(progress * 100.0);
            }
            if let Ok(mut label) = label_query.get_mut(child) {
                label.0 = label_text.clone();
            }
        }
    }
}

#[derive(Component)]
struct CastBarFill;

#[derive(Component)]
struct CastBarText;

/// Cleanup helper quando si esce dal gameplay: despawn di tutte le barre.
fn cleanup_world_cast_bars(mut commands: Commands, bars: Query<Entity, With<WorldCastBar>>) {
    for entity in bars.iter() {
        commands.entity(entity).despawn();
    }
}

/// Crea l'entity UI per una singola barra world-space.
fn spawn_world_bar(commands: &mut Commands, _theme: &UiTheme, network_id: u64) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(-60.0),
                left: Val::Px(-40.0),
                width: Val::Px(80.0),
                height: Val::Px(8.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            ZIndex(10),
            WorldCastBar {
                caster_network_id: network_id,
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.8, 1.0)),
                CastBarFill,
            ));
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: bevy::text::FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                CastBarText,
            ));
        })
        .id()
}

// Suppress unused warning per i predicati di run condition.
#[allow(dead_code)]
fn _suppress_unused() {}
