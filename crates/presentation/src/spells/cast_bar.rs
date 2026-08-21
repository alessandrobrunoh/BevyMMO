//! Screen-space cast and channel bars projected above casters.
//!
//! The bars follow the same rendering model as entity health bars: a full-screen
//! UI root owns absolute-positioned nodes, and each frame projects the caster's
//! world position through the game camera. This avoids parenting UI nodes to 3D
//! entities, which can place bars at random screen coordinates.

use bevy::prelude::*;
use std::collections::HashMap;

use bevymmo_client::network::types::ConnectedClient;
use bevymmo_gameplay::abilities::{AbilityId, BaseAbilityRegistry};
use bevymmo_network::network::protocol::{
    NetworkEntityId, Position, SpellCastEnded, SpellCastProgress,
};

use crate::game_state::{in_gameplay, not_in_gameplay};
use crate::spells::ui::{HudCooldownKey, SpellHudCooldownStarted};
use crate::ui::bar::spawn_bar;
use crate::ui::theme::UiTheme;
use lightyear::prelude::MessageReceiver;

const BAR_WIDTH: f32 = 100.0;
const BAR_HEIGHT: f32 = 14.0;
const BAR_OFFSET: Vec3 = Vec3::new(0.0, 2.55, 0.0);
const STALE_AFTER_SECONDS: f32 = 1.0;

/// Local mirror of an authoritative cast/channel snapshot.
#[derive(Debug, Clone)]
pub struct ObservedCast {
    /// The ability or spell id — carries an `AbilityId` for weapon casts and a
    /// `SpellId` string for legacy NPC/boss casts.
    pub spell_id: String,
    pub kind: u8,
    pub elapsed_seconds: f32,
    pub required_seconds: f32,
    /// Client-side extrapolation timer used between network updates.
    pub since_update_seconds: f32,
    /// Defensive expiry for dropped `SpellCastEnded` messages.
    pub stale_after_seconds: f32,
}

/// Cast/channel state observed by this client.
#[derive(Resource, Default)]
pub struct ObservedCasts(pub HashMap<u64, ObservedCast>);

#[derive(Component)]
struct CastBarRoot;

#[derive(Component)]
struct ScreenCastBar {
    caster_network_id: u64,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum CastBarKind {
    CastTime,
    Channeling,
}

#[derive(Component)]
struct CastBarParts {
    fill: Entity,
    label: Entity,
    kind: CastBarKind,
    last_left: Val,
    last_top: Val,
    last_display: Display,
    last_fill_pct: f32,
    last_label: String,
}

/// Plugin hook used by the spells plugin.
pub fn cast_bar_systems(app: &mut App) {
    app.init_resource::<ObservedCasts>();
    app.add_message::<SpellCastProgress>();
    app.add_message::<SpellCastEnded>();
    app.add_systems(
        Update,
        (
            read_cast_progress,
            read_cast_ended,
            sync_screen_cast_bars,
            // See `RenderSync`: projecting through a frame-old camera makes the
            // bar drift against the caster whenever the camera is moving.
            update_screen_cast_bars.in_set(crate::renderer::RenderSync::Project),
        )
            .chain()
            .run_if(in_gameplay),
    );
    app.add_systems(Update, cleanup_screen_cast_bars.run_if(not_in_gameplay));
}

/// Reads cast/channel snapshots from both host-client local messages and remote network receivers.
///
/// Host-client mode writes messages locally to avoid depending on a loopback
/// transport path. Dedicated clients still consume the Lightyear receiver.
fn read_cast_progress(
    time: Res<Time>,
    mut observed: ResMut<ObservedCasts>,
    mut local_messages: MessageReader<SpellCastProgress>,
    mut receivers: Query<&mut MessageReceiver<SpellCastProgress>, With<ConnectedClient>>,
) {
    let delta = time.delta_secs();
    for entry in observed.0.values_mut() {
        entry.since_update_seconds += delta;
        entry.elapsed_seconds += delta;
    }
    observed
        .0
        .retain(|_, entry| entry.since_update_seconds < entry.stale_after_seconds);

    for message in local_messages.read() {
        observe_cast_progress(&mut observed, message);
    }

    for mut receiver in receivers.iter_mut() {
        for message in receiver.receive() {
            observe_cast_progress(&mut observed, &message);
        }
    }
}

/// Reads cast-end events and starts HUD cooldowns for completed **player**
/// (weapon) casts.
///
/// Legacy NPC/boss casts are still observed visually (the bar disappears) but
/// do **not** create local player HUD cooldowns — only `AbilityId` keys that
/// exist in the `BaseAbilityRegistry` generate cooldowns.
fn read_cast_ended(
    mut observed: ResMut<ObservedCasts>,
    ability_registry: Res<BaseAbilityRegistry>,
    mut hud_cooldowns: MessageWriter<SpellHudCooldownStarted>,
    mut local_messages: MessageReader<SpellCastEnded>,
    mut receivers: Query<&mut MessageReceiver<SpellCastEnded>, With<ConnectedClient>>,
) {
    for message in local_messages.read() {
        observed.0.remove(&message.caster_network_id);
        start_cooldown_from_cast_end(&ability_registry, &mut hud_cooldowns, message);
    }

    for mut receiver in receivers.iter_mut() {
        for message in receiver.receive() {
            observed.0.remove(&message.caster_network_id);
            start_cooldown_from_cast_end(&ability_registry, &mut hud_cooldowns, &message);
        }
    }
}

/// Starts a HUD cooldown when a server-authoritative cast/channel ends successfully.
///
/// Only emits a cooldown for **weapon abilities** found in the registry (the
/// player's weapon gestures). Legacy spell ids from NPCs/bosses are silently
/// skipped so they never pollute the player's HUD state.
fn start_cooldown_from_cast_end(
    ability_registry: &BaseAbilityRegistry,
    hud_cooldowns: &mut MessageWriter<SpellHudCooldownStarted>,
    message: &SpellCastEnded,
) {
    if !message.completed {
        return;
    }

    // Try weapon ability first — this is the player path.
    let ability_id = AbilityId::new(message.spell_id.clone());
    if let Some(ability) = ability_registry.get(&ability_id) {
        let cooldown_seconds = ability.base_params().cooldown;
        if cooldown_seconds <= 0.0 {
            return;
        }
        hud_cooldowns.write(SpellHudCooldownStarted {
            key: HudCooldownKey::Ability(ability_id),
            cooldown_seconds,
        });
    }

    // Legacy spell id (NPC/boss) — do not create a player HUD cooldown.
    // The visual bar is already removed by the caller; this is just the cooldown path.
}

/// Stores the latest authoritative cast snapshot per caster.
fn observe_cast_progress(observed: &mut ObservedCasts, message: &SpellCastProgress) {
    observed.0.insert(
        message.caster_network_id,
        ObservedCast {
            spell_id: message.spell_id.clone(),
            kind: message.kind,
            elapsed_seconds: message.elapsed_seconds,
            required_seconds: message.required_seconds,
            since_update_seconds: 0.0,
            stale_after_seconds: STALE_AFTER_SECONDS,
        },
    );
}

/// Spawns/despawns one screen-space bar per observed caster.
///
/// The actual position and fill are updated separately so this system only
/// reacts to lifecycle changes.
fn sync_screen_cast_bars(
    mut commands: Commands,
    theme: Res<UiTheme>,
    observed: Res<ObservedCasts>,
    root_query: Query<Entity, With<CastBarRoot>>,
    bars: Query<(Entity, &ScreenCastBar)>,
) {
    let root = get_or_spawn_root(&mut commands, &root_query);

    for (bar_entity, bar) in bars.iter() {
        if observed.0.contains_key(&bar.caster_network_id) {
            continue;
        }
        commands.entity(bar_entity).despawn();
    }

    for (&network_id, cast) in observed.0.iter() {
        if bars
            .iter()
            .any(|(_, bar)| bar.caster_network_id == network_id)
        {
            continue;
        }
        spawn_screen_bar(&mut commands, root, network_id, cast_bar_kind(cast), &theme);
    }
}

/// Projects bars above their casters and updates label/fill values.
///
/// CastTime bars fill left-to-right. Channeling bars drain right-to-left when
/// the channel has a finite duration.
fn update_screen_cast_bars(
    camera_query: Query<(&Camera, &Transform), With<Camera3d>>,
    caster_query: Query<(&NetworkEntityId, &Position, Option<&Transform>), Without<Camera3d>>,
    observed: Res<ObservedCasts>,
    mut bar_query: Query<(&ScreenCastBar, &mut Node, &mut CastBarParts)>,
    mut fill_query: Query<&mut Node, Without<ScreenCastBar>>,
    mut text_query: Query<&mut Text>,
    ui_scale: Res<UiScale>,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let camera_transform = crate::renderer::camera_view(camera_transform);

    let scale_factor = ui_scale.0;

    for (bar, mut node, mut parts) in bar_query.iter_mut() {
        let Some(cast) = observed.0.get(&bar.caster_network_id) else {
            set_bar_display(&mut node, &mut parts, Display::None);
            continue;
        };

        let Some((_, caster_position, rendered)) = caster_query
            .iter()
            .find(|(network_id, _, _)| network_id.0 == bar.caster_network_id)
        else {
            set_bar_display(&mut node, &mut parts, Display::None);
            continue;
        };

        // Anchor to the rendered transform, not the fixed-step `Position`: the
        // mesh is drawn from the former, so using the latter floats the bar a
        // tick ahead of the caster it belongs to.
        let anchor = rendered.map(|t| t.translation).unwrap_or(caster_position.0);
        let world_pos = anchor + BAR_OFFSET;
        let Ok(viewport_pos) = camera.world_to_viewport(&camera_transform, world_pos) else {
            set_bar_display(&mut node, &mut parts, Display::None);
            continue;
        };

        let scaled_viewport_pos = viewport_pos / scale_factor;
        set_bar_position(&mut node, &mut parts, scaled_viewport_pos);
        update_bar_content(cast, &mut parts, &mut fill_query, &mut text_query);
    }
}

fn get_or_spawn_root(commands: &mut Commands, query: &Query<Entity, With<CastBarRoot>>) -> Entity {
    if let Ok(entity) = query.single() {
        return entity;
    }

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            CastBarRoot,
        ))
        .id()
}

fn spawn_screen_bar(
    commands: &mut Commands,
    root: Entity,
    caster_network_id: u64,
    kind: CastBarKind,
    theme: &UiTheme,
) {
    let fill_color = match kind {
        CastBarKind::CastTime => Color::srgb(1.0, 0.62, 0.15),
        CastBarKind::Channeling => Color::srgb(0.25, 0.65, 1.0),
    };
    let bar_entity = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                ..default()
            },
            ScreenCastBar { caster_network_id },
        ))
        .id();

    let (bar_body, fill_entity) = spawn_bar(
        commands,
        bar_entity,
        0.0,
        1.0,
        Vec2::new(BAR_WIDTH, BAR_HEIGHT),
        Color::srgba(0.0, 0.0, 0.0, 0.72),
        fill_color,
    );
    commands.entity(fill_entity).insert(CastBarFill);
    let label_entity = commands
        .spawn((
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(10.0),
                ..default()
            },
            TextColor(theme.text_color),
        ))
        .id();
    commands.entity(bar_body).add_child(label_entity);

    commands.entity(bar_entity).insert(CastBarParts {
        fill: fill_entity,
        label: label_entity,
        kind,
        last_left: Val::Auto,
        last_top: Val::Auto,
        last_display: Display::None,
        last_fill_pct: -1.0,
        last_label: String::new(),
    });
    commands.entity(root).add_child(bar_entity);
}

#[derive(Component)]
struct CastBarFill;

fn cast_bar_kind(cast: &ObservedCast) -> CastBarKind {
    // SpellCastProgress mapping: Instant/CastTime = 0, Channeling = 1
    match cast.kind {
        1 => CastBarKind::Channeling,
        _ => CastBarKind::CastTime,
    }
}

fn set_bar_position(node: &mut Node, parts: &mut CastBarParts, viewport_pos: Vec2) {
    let left = Val::Px(viewport_pos.x - BAR_WIDTH * 0.5);
    let top = Val::Px(viewport_pos.y - 48.0);

    if parts.last_left != left {
        node.left = left;
        parts.last_left = left;
    }
    if parts.last_top != top {
        node.top = top;
        parts.last_top = top;
    }
    set_bar_display(node, parts, Display::Flex);
}

fn set_bar_display(node: &mut Node, parts: &mut CastBarParts, display: Display) {
    if parts.last_display == display {
        return;
    }
    node.display = display;
    parts.last_display = display;
}

fn update_bar_content(
    cast: &ObservedCast,
    parts: &mut CastBarParts,
    fill_query: &mut Query<&mut Node, Without<ScreenCastBar>>,
    text_query: &mut Query<&mut Text>,
) {
    let fill_pct = cast_fill_pct(cast);
    if (parts.last_fill_pct - fill_pct).abs() > 0.25 {
        if let Ok(mut fill_node) = fill_query.get_mut(parts.fill) {
            fill_node.width = Val::Percent(fill_pct);
        }
        parts.last_fill_pct = fill_pct;
    }

    let label = cast_label(cast, parts.kind);
    if parts.last_label == label {
        return;
    }
    if let Ok(mut text) = text_query.get_mut(parts.label) {
        text.0 = label.clone();
    }
    parts.last_label = label;
}

fn cast_fill_pct(cast: &ObservedCast) -> f32 {
    if cast.required_seconds <= 0.0 {
        return 100.0;
    }

    let progress = (cast.elapsed_seconds / cast.required_seconds).clamp(0.0, 1.0);
    match cast.kind {
        1 => {
            // Channeling drains right-to-left.
            (1.0 - progress) * 100.0
        }
        _ => {
            // CastTime and Charge fill left-to-right.
            progress * 100.0
        }
    }
}

fn cast_label(cast: &ObservedCast, kind: CastBarKind) -> String {
    let remaining = (cast.required_seconds - cast.elapsed_seconds).max(0.0);
    match kind {
        CastBarKind::CastTime => format!("Cast {} {:.1}s", cast.spell_id, remaining),
        CastBarKind::Channeling => format!("Channel {} {:.1}s", cast.spell_id, remaining),
    }
}

fn cleanup_screen_cast_bars(
    mut commands: Commands,
    roots: Query<Entity, With<CastBarRoot>>,
    mut observed: ResMut<ObservedCasts>,
) {
    observed.0.clear();
    for root in roots.iter() {
        commands.entity(root).despawn();
    }
}
