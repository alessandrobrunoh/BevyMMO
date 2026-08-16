//! Sistemi della UI flottante.
//!
//! Aggiornamenti divisi in tre fasi concatenate:
//! 1. `update_floating_ui_position` — proietta la posizione mondo in viewport e
//!    centra lo stack nome+barra sopra il punto proiettato.
//! 2. `update_floating_ui_content` — aggiorna nome/fill/testo HP tramite
//!    [`EntityBarParts`] con cache per saltare scritture invariate (niente
//!    `Children` walking, niente realloc/relayout per frame).
//! 3. `cleanup_floating_ui` — despawn quando il target non è più valido.
//!
//! Fuori dal gameplay, `cleanup_floating_ui_root` rimuove la root UI e resetta
//! `FloatingUiAttached` sui target, così il re-entry può ri-spawnare le barre.

use bevymmo_gameplay::entity::components::{EntityKind, PlayerName};
use bevymmo_network::network::protocol::Position;
use bevymmo_gameplay::stats::components::VitalStats;

use crate::game_state::{GameScreen, Screen};
use crate::ui::theme::UiTheme;
use bevy::color::Color;
use bevy::prelude::*;

use super::{spawn_entity_bar, EntityBarParts, FloatingUi};

/// Root UI Node per tutta la UI flottante.
#[derive(Component)]
pub struct FloatingUiRoot;

/// Marker: entità di gioco che possiede già una UI flottante.
#[derive(Component)]
pub struct FloatingUiAttached;

/// Distanza (in px² di viewport) sotto la quale il container non viene toccato.
///
/// 0.02 px: abbastanza da evitare il relayout di un bersaglio immobile, troppo
/// poco per essere percepita come uno scatto su un bersaglio in movimento.
const VIEWPORT_EPSILON_SQUARED: f32 = 0.0004;

fn get_or_spawn_root(
    commands: &mut Commands,
    query: &Query<Entity, With<FloatingUiRoot>>,
) -> Entity {
    if let Ok(entity) = query.single() {
        entity
    } else {
        commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                FloatingUiRoot,
            ))
            .id()
    }
}

pub fn spawn_ui_for_new_entities(
    mut commands: Commands,
    root_query: Query<Entity, With<FloatingUiRoot>>,
    theme: Res<UiTheme>,
    new_entities: Query<
        Entity,
        (
            With<Position>,
            With<VitalStats>,
            Without<FloatingUiAttached>,
        ),
    >,
) {
    if new_entities.is_empty() {
        return;
    }

    let root = get_or_spawn_root(&mut commands, &root_query);

    for entity in new_entities.iter() {
        spawn_entity_bar(&mut commands, root, entity, &theme);
        commands.entity(entity).insert(FloatingUiAttached);
    }
}

/// Fase 1 — aggiorna posizione e visibilità del nodo container.
///
/// Usa `With<Camera3d>` per selezionare la camera di gioco: il client ha anche
/// una `Camera2d` per la UI e un filtro senza marker farebbe fallire `single()`,
/// lasciando tutte le barre alla posizione di default.
///
/// Lo stack è centrato orizzontalmente e ancorato col fondo sopra il punto
/// proiettato: `left = viewport.x - BAR_WIDTH/2`,
/// `top  = viewport.y - STACK_HEIGHT`.
pub fn update_floating_ui_position(
    camera_query: Query<(&Camera, &Transform), With<Camera3d>>,
    target_query: Query<(&Position, Option<&Transform>), Without<Camera3d>>,
    ui_scale: Res<UiScale>,
    mut ui_query: Query<(&mut FloatingUi, &mut Node)>,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let camera_transform = crate::renderer::camera_view(camera_transform);

    let scale_factor = ui_scale.0;

    for (mut floating_ui, mut node) in ui_query.iter_mut() {
        let Ok((pos, rendered)) = target_query.get(floating_ui.target) else {
            continue;
        };

        // Anchor to the *rendered* transform, falling back to the simulated
        // position only before the renderer has given the entity one. The mesh
        // is drawn from that transform, and `Position` steps on the fixed
        // schedule: reading the latter here made the bar hop a tick ahead of
        // the character it is supposed to sit on.
        let anchor = rendered.map(|t| t.translation).unwrap_or(pos.0);
        let world_pos = anchor + floating_ui.offset;
        let Ok(viewport_pos) = camera.world_to_viewport(&camera_transform, world_pos) else {
            // Dietro la camera: nascondi senza toccare left/top.
            if node.display != Display::None {
                node.display = Display::None;
            }
            floating_ui.last_viewport = None;
            continue;
        };

        // Skip scrittura se la posizione viewport non è cambiata.
        //
        // È solo una guardia contro il relayout di un target *fermo*. La
        // tolleranza era 0.5 px, che quantizzava lo scorrimento della barra in
        // salti di mezzo pixel ben visibili mentre il player cammina: il
        // personaggio scorreva liscio, la barra sopra di lui no.
        if floating_ui
            .last_viewport
            .is_some_and(|last| (last - viewport_pos).length_squared() < VIEWPORT_EPSILON_SQUARED)
        {
            continue;
        }
        floating_ui.last_viewport = Some(viewport_pos);

        let new_left = Val::Px((viewport_pos.x / scale_factor) - super::plugin::BAR_WIDTH * 0.5);
        let new_top = Val::Px((viewport_pos.y / scale_factor) - super::plugin::STACK_HEIGHT);
        let new_display = Display::Flex;

        if node.left != new_left {
            node.left = new_left;
        }
        if node.top != new_top {
            node.top = new_top;
        }
        if node.display != new_display {
            node.display = new_display;
        }
    }
}

/// Fase 2 — aggiorna contenuto (nome, fill, testo HP, colore fill) tramite riferimenti diretti,
/// saltando le scritture invariate grazie alla cache in [`EntityBarParts`].
pub fn update_floating_ui_content(
    changed_targets: Query<
        Entity,
        Or<(
            Changed<VitalStats>,
            Changed<PlayerName>,
            Changed<EntityKind>,
        )>,
    >,
    target_query: Query<(&VitalStats, Option<&PlayerName>, Option<&EntityKind>)>,
    theme: Res<UiTheme>,
    mut ui_query: Query<(&FloatingUi, &mut EntityBarParts)>,
    mut text_query: Query<&mut Text>,
    mut node_query: Query<&mut Node>,
    mut bg_query: Query<&mut BackgroundColor>,
) {
    for (floating_ui, mut parts) in ui_query.iter_mut() {
        // Processa solo se il target ha componenti variati o se la parte è ancora ininizializzata (last_fill_pct < 0.0)
        if parts.last_fill_pct >= 0.0 && !changed_targets.contains(floating_ui.target) {
            continue;
        }

        let Ok((vital, name, entity_kind)) = target_query.get(floating_ui.target) else {
            continue;
        };

        // Nome: scrittura + cache solo se il valore è cambiato.
        let new_name = name
            .map(|n| n.0.clone())
            .unwrap_or_else(|| "Entity".to_string());
        if parts.last_name != new_name {
            let name_entity = parts.name_text;
            if let Ok(mut text) = text_query.get_mut(name_entity) {
                text.0 = new_name.clone();
            }
            parts.last_name = new_name;
        }

        // Fill: clampa [0,1] -> [0,100], scrivi solo se la percentuale è cambiata.
        let new_fill_pct =
            (vital.current_health / vital.max_health.max(0.1)).clamp(0.0, 1.0) * 100.0;
        if parts.last_fill_pct != new_fill_pct {
            let fill_entity = parts.hp_fill;
            if let Ok(mut fill_node) = node_query.get_mut(fill_entity) {
                fill_node.width = Val::Percent(new_fill_pct);
            }
            parts.last_fill_pct = new_fill_pct;
        }

        // Testo HP: "current/max" intero, scrivi solo se la stringa è cambiata.
        let new_hp_text = format!(
            "{}/{}",
            vital.current_health as i32, vital.max_health as i32
        );
        if parts.last_hp_text != new_hp_text {
            let hp_text_entity = parts.hp_text;
            if let Ok(mut text) = text_query.get_mut(hp_text_entity) {
                text.0 = new_hp_text.clone();
            }
            parts.last_hp_text = new_hp_text;
        }

        // Colore fill: in base a EntityKind, scrivi solo se il colore è cambiato.
        let new_fill_color = get_hp_fill_color(entity_kind, &theme);
        let fill_entity = parts.hp_fill;
        if let Ok(mut bg) = bg_query.get_mut(fill_entity) {
            if bg.0 != new_fill_color {
                bg.0 = new_fill_color;
            }
        }
    }
}

/// Fase 3 — despawn quando il target non esiste più o ha perso `Position`.
pub fn cleanup_floating_ui(
    mut commands: Commands,
    target_query: Query<&Position>,
    ui_query: Query<(Entity, &FloatingUi)>,
) {
    for (ui_entity, floating_ui) in ui_query.iter() {
        if target_query.get(floating_ui.target).is_err() {
            commands.entity(ui_entity).despawn();
        }
    }
}

/// Fuori dal gameplay: despawn della root UI flottante (e dei figli) e reset
/// del marker `FloatingUiAttached` sui target, così un re-entry ri-spawna le
/// barre pulite invece di lasciare UI orfane dal match precedente.
pub fn cleanup_floating_ui_root(
    mut commands: Commands,
    roots: Query<Entity, With<FloatingUiRoot>>,
    attached: Query<Entity, With<FloatingUiAttached>>,
) {
    let mut despawned_any = false;
    for root in roots.iter() {
        commands.entity(root).despawn();
        despawned_any = true;
    }
    if despawned_any {
        for entity in attached.iter() {
            commands.entity(entity).remove::<FloatingUiAttached>();
        }
    }
}

/// Condizione di esecuzione: il client NON è in una schermata di gameplay.
pub(crate) fn not_in_gameplay(screen: Res<GameScreen>) -> bool {
    !matches!(screen.0, Screen::InGame | Screen::Paused)
}

/// Determina il colore della barra HP in base al tipo di entità.
///
/// - Player: verde-blu
/// - Friendly: verde
/// - Neutral: giallo
/// - Hostile: rosso
/// - None: fallback a theme.hp_fill
fn get_hp_fill_color(entity_kind: Option<&EntityKind>, theme: &UiTheme) -> Color {
    match entity_kind {
        Some(EntityKind::Player) => Color::srgb(0.3, 0.8, 0.5),
        Some(EntityKind::Friendly) => Color::srgb(0.2, 0.9, 0.3),
        Some(EntityKind::Neutral) => Color::srgb(0.9, 0.9, 0.2),
        Some(EntityKind::Hostile) => Color::srgb(0.9, 0.1, 0.1),
        None => theme.hp_fill,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::UiTheme;

    /// App minima con solo le risorse/componenti necessarie; nessun renderer.
    fn content_app() -> App {
        let mut app = App::new();
        app.init_resource::<UiTheme>();
        app.init_resource::<GameScreen>();
        app.add_systems(
            Update,
            (spawn_ui_for_new_entities, update_floating_ui_content).chain(),
        );
        app
    }

    fn root_count(world: &mut World) -> usize {
        world.query::<&FloatingUiRoot>().iter(world).count()
    }

    #[test]
    fn container_is_hidden_until_first_projection() {
        let mut app = content_app();
        app.world_mut().spawn((
            Position(Vec3::ZERO),
            VitalStats {
                current_health: 50.0,
                max_health: 50.0,
                max_mana: 40.0,
                mana_regeneration: 2.0,
            },
        ));
        app.update();

        let mut q = app.world_mut().query_filtered::<&Node, With<FloatingUi>>();
        let node = q.single(app.world()).expect("floating UI spawned");
        assert_eq!(
            node.display,
            Display::None,
            "il container deve restare nascosto finché la posizione non è proiettata"
        );
    }

    #[test]
    fn fill_clamps_when_health_exceeds_max() {
        let mut app = content_app();
        app.world_mut().spawn((
            Position(Vec3::ZERO),
            VitalStats {
                current_health: 150.0,
                max_health: 100.0,
                max_mana: 40.0,
                mana_regeneration: 2.0,
            },
        ));
        app.update();

        let mut q = app.world_mut().query::<&EntityBarParts>();
        let parts = q.single(app.world()).expect("parts");
        assert_eq!(parts.last_fill_pct, 100.0);
    }

    #[test]
    fn fill_clamps_negative_health_to_zero() {
        let mut app = content_app();
        app.world_mut().spawn((
            Position(Vec3::ZERO),
            VitalStats {
                current_health: -10.0,
                max_health: 100.0,
                max_mana: 40.0,
                mana_regeneration: 2.0,
            },
        ));
        app.update();

        let mut q = app.world_mut().query::<&EntityBarParts>();
        let parts = q.single(app.world()).expect("parts");
        assert_eq!(parts.last_fill_pct, 0.0);
    }

    #[test]
    fn content_writes_initial_values_on_first_update() {
        let mut app = content_app();
        app.world_mut().spawn((
            Position(Vec3::ZERO),
            VitalStats {
                current_health: 25.0,
                max_health: 100.0,
                max_mana: 40.0,
                mana_regeneration: 2.0,
            },
            PlayerName("Alice".to_string()),
        ));
        app.update();

        let mut q = app.world_mut().query::<&EntityBarParts>();
        let parts = q.single(app.world()).expect("parts");
        assert_eq!(parts.last_name, "Alice");
        assert_eq!(parts.last_hp_text, "25/100");
        assert_eq!(parts.last_fill_pct, 25.0);
    }

    #[test]
    fn content_cache_is_stable_across_identical_updates() {
        let mut app = content_app();
        app.world_mut().spawn((
            Position(Vec3::ZERO),
            VitalStats {
                current_health: 50.0,
                max_health: 100.0,
                max_mana: 40.0,
                mana_regeneration: 2.0,
            },
            PlayerName("Bob".to_string()),
        ));
        app.update();

        let (name, hp, fill) = {
            let mut q = app.world_mut().query::<&EntityBarParts>();
            let p = q.single(app.world()).unwrap();
            (p.last_name.clone(), p.last_hp_text.clone(), p.last_fill_pct)
        };

        // Stessi dati: la cache non deve cambiare.
        app.update();
        let mut q = app.world_mut().query::<&EntityBarParts>();
        let p = q.single(app.world()).unwrap();
        assert_eq!(p.last_name, name);
        assert_eq!(p.last_hp_text, hp);
        assert_eq!(p.last_fill_pct, fill);
    }

    #[test]
    fn content_updates_when_health_changes() {
        let mut app = content_app();
        let target = app
            .world_mut()
            .spawn((
                Position(Vec3::ZERO),
                VitalStats {
                    current_health: 50.0,
                    max_health: 100.0,
                    max_mana: 40.0,
                    mana_regeneration: 2.0,
                },
            ))
            .id();
        app.update();

        // Mutazione del dato replicato: al prossimo update la cache si invalida.
        app.world_mut()
            .entity_mut(target)
            .get_mut::<VitalStats>()
            .unwrap()
            .current_health = 25.0;
        app.update();

        let mut q = app.world_mut().query::<&EntityBarParts>();
        let parts = q.single(app.world()).unwrap();
        assert_eq!(parts.last_hp_text, "25/100");
        assert_eq!(parts.last_fill_pct, 25.0);
    }

    #[test]
    fn name_falls_back_to_entity_when_missing() {
        let mut app = content_app();
        app.world_mut().spawn((
            Position(Vec3::ZERO),
            VitalStats {
                current_health: 100.0,
                max_health: 100.0,
                max_mana: 40.0,
                mana_regeneration: 2.0,
            }, // niente PlayerName
        ));
        app.update();

        let mut q = app.world_mut().query::<&EntityBarParts>();
        let parts = q.single(app.world()).unwrap();
        assert_eq!(parts.last_name, "Entity");
    }

    #[test]
    fn root_is_despawned_when_leaving_gameplay() {
        let mut app = App::new();
        app.init_resource::<GameScreen>();
        app.init_resource::<UiTheme>();
        app.add_systems(
            Update,
            spawn_ui_for_new_entities.run_if(crate::ui::systems::in_gameplay),
        );
        app.add_systems(Update, cleanup_floating_ui_root.run_if(not_in_gameplay));

        // Enter gameplay: spawn di un'entità e della root.
        app.world_mut().resource_mut::<GameScreen>().0 = Screen::InGame;
        app.world_mut().spawn((
            Position(Vec3::ZERO),
            VitalStats {
                current_health: 50.0,
                max_health: 50.0,
                max_mana: 40.0,
                mana_regeneration: 2.0,
            },
        ));
        app.update();
        assert_eq!(root_count(app.world_mut()), 1);

        // Leave gameplay: la root e i figli vengono despawnati.
        app.world_mut().resource_mut::<GameScreen>().0 = Screen::MainMenu;
        app.update();
        assert_eq!(
            root_count(app.world_mut()),
            0,
            "la root UI flottante deve essere despawnata fuori dal gameplay"
        );
    }

    #[test]
    fn floating_ui_attached_is_cleared_on_root_cleanup_so_reentry_respawns() {
        let mut app = App::new();
        app.init_resource::<GameScreen>();
        app.init_resource::<UiTheme>();
        app.add_systems(
            Update,
            spawn_ui_for_new_entities.run_if(crate::ui::systems::in_gameplay),
        );
        app.add_systems(Update, cleanup_floating_ui_root.run_if(not_in_gameplay));

        app.world_mut().resource_mut::<GameScreen>().0 = Screen::InGame;
        let target = app
            .world_mut()
            .spawn((
                Position(Vec3::ZERO),
                VitalStats {
                    current_health: 50.0,
                    max_health: 50.0,
                    max_mana: 40.0,
                    mana_regeneration: 2.0,
                },
            ))
            .id();
        app.update();
        assert!(app
            .world()
            .entity(target)
            .get::<FloatingUiAttached>()
            .is_some());

        // Leave + cleanup: marker rimosso.
        app.world_mut().resource_mut::<GameScreen>().0 = Screen::MainMenu;
        app.update();
        assert!(
            app.world()
                .entity(target)
                .get::<FloatingUiAttached>()
                .is_none(),
            "FloatingUiAttached deve essere rimosso per permettere il re-spawn"
        );
    }

    /// Contratto geometrico: le costanti di centratura devono riflettere
    /// l'effettiva dimensione della barra spawnata (100x14).
    #[test]
    fn centering_constants_match_bar_geometry() {
        assert_eq!(crate::ui::entity_bar::plugin::BAR_WIDTH, 100.0);
        assert_eq!(crate::ui::entity_bar::plugin::BAR_HEIGHT, 14.0);
        // `STACK_HEIGHT > BAR_HEIGHT` è verificato a compile time da un
        // `const _: () = assert!(..)` in `plugin.rs`.
    }

    #[test]
    fn hp_fill_color_is_green_for_player() {
        let color = get_hp_fill_color(Some(&EntityKind::Player), &UiTheme::default());
        assert_eq!(color, Color::srgb(0.3, 0.8, 0.5));
    }

    #[test]
    fn hp_fill_color_is_green_for_friendly() {
        let color = get_hp_fill_color(Some(&EntityKind::Friendly), &UiTheme::default());
        assert_eq!(color, Color::srgb(0.2, 0.9, 0.3));
    }

    #[test]
    fn hp_fill_color_is_yellow_for_neutral() {
        let color = get_hp_fill_color(Some(&EntityKind::Neutral), &UiTheme::default());
        assert_eq!(color, Color::srgb(0.9, 0.9, 0.2));
    }

    #[test]
    fn hp_fill_color_is_red_for_hostile() {
        let color = get_hp_fill_color(Some(&EntityKind::Hostile), &UiTheme::default());
        assert_eq!(color, Color::srgb(0.9, 0.1, 0.1));
    }

    #[test]
    fn hp_fill_color_falls_back_to_theme_when_no_entity_kind() {
        let theme = UiTheme::default();
        let color = get_hp_fill_color(None, &theme);
        assert_eq!(color, theme.hp_fill);
    }
}
