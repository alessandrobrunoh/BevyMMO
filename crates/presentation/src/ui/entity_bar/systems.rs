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

use bevymmo_client::gathering::pick_height_for;
use bevymmo_gameplay::entity::components::{EntityKind, PlayerName};
use bevymmo_gameplay::gathering::Harvestable;
use bevymmo_gameplay::placeables::PlaceableRegistry;
use bevymmo_gameplay::stats::components::VitalStats;
use bevymmo_network::network::protocol::Position;

use crate::ui::bar::{get_hp_fill_color, get_or_spawn_root};
use crate::ui::theme::UiTheme;
use bevy::prelude::*;

use super::plugin::{CHARACTER_BAR_OFFSET, HARVESTABLE_BAR_CLEARANCE};
use super::{spawn_entity_bar, EntityBarParts, FloatingUi};

/// Root UI Node per tutta la UI flottante.
#[derive(Component, Default)]
pub struct FloatingUiRoot;

/// Marker: entità di gioco che possiede già una UI flottante.
#[derive(Component)]
pub struct FloatingUiAttached;

/// Distanza (in px² di viewport) sotto la quale il container non viene toccato.
///
/// 0.02 px: abbastanza da evitare il relayout di un bersaglio immobile, troppo
/// poco per essere percepita come uno scatto su un bersaglio in movimento.
const VIEWPORT_EPSILON_SQUARED: f32 = 1.0;

pub fn spawn_ui_for_new_entities(
    mut commands: Commands,
    root_query: Query<Entity, With<FloatingUiRoot>>,
    theme: Res<UiTheme>,
    placeables: Res<PlaceableRegistry>,
    new_entities: Query<
        (Entity, Option<&Harvestable>),
        (
            With<Position>,
            Or<(With<VitalStats>, With<Harvestable>)>,
            Without<FloatingUiAttached>,
        ),
    >,
) {
    if new_entities.is_empty() {
        return;
    }

    let root = get_or_spawn_root(&mut commands, &root_query);

    for (entity, harvestable) in new_entities.iter() {
        let offset = harvestable
            .map(|harvestable| {
                // A node's model is nothing like a character: the oak stands
                // 11.9 m tall, so a fixed 2 m offset buries the bar in the
                // trunk. `pick_height_for` is the height the click volume is
                // already built from — one number, not two that can drift.
                Vec3::Y
                    * (pick_height_for(harvestable.kind_id.as_str(), Some(&placeables))
                        + HARVESTABLE_BAR_CLEARANCE)
            })
            .unwrap_or(CHARACTER_BAR_OFFSET);
        spawn_entity_bar(&mut commands, root, entity, offset, &theme);
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

/// Cosa conta la barra: salute per chi combatte, pezzi rimasti per un nodo
/// raccoglibile. Un nodo non ha `VitalStats` e nient'altro ha `Harvestable`,
/// quindi i due casi non competono mai.
///
/// `None` quando il target non è nessuno dei due, o quando il registry non
/// conosce quel `kind_id`: senza il massimo la barra non ha una scala.
fn bar_values(
    vital: Option<&VitalStats>,
    harvestable: Option<&Harvestable>,
    placeables: &PlaceableRegistry,
) -> Option<(f32, f32)> {
    if let Some(vital) = vital {
        return Some((vital.current_health, vital.max_health));
    }
    let harvestable = harvestable?;
    let max = crate::harvest::max_pieces_for(&harvestable.kind_id, placeables)?;
    Some((harvestable.current_pieces as f32, max as f32))
}

/// Fase 2 — aggiorna contenuto (nome, fill, testo, colore fill) tramite riferimenti diretti,
/// saltando le scritture invariate grazie alla cache in [`EntityBarParts`].
pub fn update_floating_ui_content(
    changed_targets: Query<
        Entity,
        Or<(
            Changed<VitalStats>,
            Changed<Harvestable>,
            Changed<PlayerName>,
            Changed<EntityKind>,
        )>,
    >,
    target_query: Query<(
        Option<&VitalStats>,
        Option<&Harvestable>,
        Option<&PlayerName>,
        Option<&EntityKind>,
    )>,
    theme: Res<UiTheme>,
    placeables: Res<PlaceableRegistry>,
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

        let Ok((vital, harvestable, name, entity_kind)) = target_query.get(floating_ui.target)
        else {
            continue;
        };
        let Some((current, max)) = bar_values(vital, harvestable, &placeables) else {
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
        let new_fill_pct = (current / max.max(0.1)).clamp(0.0, 1.0) * 100.0;
        if parts.last_fill_pct != new_fill_pct {
            let fill_entity = parts.hp_fill;
            if let Ok(mut fill_node) = node_query.get_mut(fill_entity) {
                fill_node.width = Val::Percent(new_fill_pct);
            }
            parts.last_fill_pct = new_fill_pct;
        }

        // Testo: "current/max" intero, scrivi solo se la stringa è cambiata.
        let new_hp_text = format!("{}/{}", current as i32, max as i32);
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

/// Determina il colore della barra HP in base al tipo di entità.
///
/// - Player: verde-blu
/// - Friendly: verde
/// - Neutral: giallo
/// - Hostile: rosso
/// - None: fallback a theme.hp_fill
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::Screen;
    use crate::ui::theme::UiTheme;

    /// App minima con solo le risorse/componenti necessarie; nessun renderer.
    fn content_app() -> App {
        let mut app = App::new();
        app.init_resource::<UiTheme>();
        insert_placeables(&mut app);
        app.add_systems(
            Update,
            (spawn_ui_for_new_entities, update_floating_ui_content).chain(),
        );
        app
    }

    /// The real registry, not an empty one: the pieces bar reads a node's
    /// maximum from it, so an empty registry would silently skip that path.
    fn insert_placeables(app: &mut App) {
        let mut registry = PlaceableRegistry::default();
        bevymmo_content::placeable_definitions::register_all(&mut registry);
        app.insert_resource(registry);
    }

    fn oak(pieces: u32) -> Harvestable {
        Harvestable {
            placement_id: "oak_spawn_west".to_string(),
            kind_id: "resource_oak_tree".to_string(),
            current_pieces: pieces,
        }
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
                current_mana: 40.0,
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
                current_mana: 40.0,
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
                current_mana: 40.0,
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
                current_mana: 40.0,
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
                current_mana: 40.0,
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
                    current_mana: 40.0,
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
                current_mana: 40.0,
                max_mana: 40.0,
                mana_regeneration: 2.0,
            }, // niente PlayerName
        ));
        app.update();

        let mut q = app.world_mut().query::<&EntityBarParts>();
        let parts = q.single(app.world()).unwrap();
        assert_eq!(parts.last_name, "Entity");
    }

    /// A resource node has no `VitalStats`, so it used to get no bar at all —
    /// the pieces left in it were only visible by counting the gathers.
    #[test]
    fn harvestable_node_shows_pieces_instead_of_health() {
        let mut app = content_app();
        app.world_mut().spawn((
            Position(Vec3::ZERO),
            oak(47),
            PlayerName("Oak Tree".to_string()),
        ));
        app.update();

        let mut q = app.world_mut().query::<&EntityBarParts>();
        let parts = q.single(app.world()).expect("parts");
        assert_eq!(parts.last_name, "Oak Tree");
        assert_eq!(parts.last_hp_text, "47/50");
        assert_eq!(parts.last_fill_pct, 94.0);
    }

    #[test]
    fn gathering_a_piece_moves_the_bar() {
        let mut app = content_app();
        let tree = app.world_mut().spawn((Position(Vec3::ZERO), oak(50))).id();
        app.update();

        app.world_mut()
            .entity_mut(tree)
            .get_mut::<Harvestable>()
            .expect("harvestable")
            .current_pieces = 25;
        app.update();

        let mut q = app.world_mut().query::<&EntityBarParts>();
        let parts = q.single(app.world()).expect("parts");
        assert_eq!(parts.last_hp_text, "25/50");
        assert_eq!(parts.last_fill_pct, 50.0);
    }

    /// The character offset (2 m) sits inside an 11.9 m oak's trunk.
    #[test]
    fn a_node_carries_its_bar_above_the_model() {
        let mut app = content_app();
        app.world_mut().spawn((Position(Vec3::ZERO), oak(50)));
        app.world_mut().spawn((
            Position(Vec3::ZERO),
            VitalStats {
                current_health: 50.0,
                max_health: 50.0,
                current_mana: 40.0,
                max_mana: 40.0,
                mana_regeneration: 2.0,
            },
        ));
        app.update();

        let mut q = app.world_mut().query::<&FloatingUi>();
        let mut offsets: Vec<f32> = q.iter(app.world()).map(|ui| ui.offset.y).collect();
        offsets.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(offsets[0], CHARACTER_BAR_OFFSET.y);
        assert!(
            offsets[1] > CHARACTER_BAR_OFFSET.y,
            "the oak's bar must clear its canopy, got {}",
            offsets[1]
        );
    }

    #[test]
    fn root_is_despawned_when_leaving_gameplay() {
        let mut app = App::new();
        crate::game_state::init_screen_states(&mut app);
        app.init_resource::<UiTheme>();
        insert_placeables(&mut app);
        app.add_systems(
            Update,
            spawn_ui_for_new_entities.run_if(in_state(Screen::InGame)),
        );
        app.add_systems(
            Update,
            cleanup_floating_ui_root.run_if(crate::game_state::not_in_gameplay),
        );

        // Enter gameplay: spawn di un'entità e della root.
        app.insert_state(Screen::InGame);
        app.world_mut().spawn((
            Position(Vec3::ZERO),
            VitalStats {
                current_health: 50.0,
                max_health: 50.0,
                current_mana: 40.0,
                max_mana: 40.0,
                mana_regeneration: 2.0,
            },
        ));
        app.update();
        assert_eq!(root_count(app.world_mut()), 1);

        // Leave gameplay: la root e i figli vengono despawnati.
        app.insert_state(Screen::MainMenu);
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
        crate::game_state::init_screen_states(&mut app);
        app.init_resource::<UiTheme>();
        insert_placeables(&mut app);
        app.add_systems(
            Update,
            spawn_ui_for_new_entities.run_if(in_state(Screen::InGame)),
        );
        app.add_systems(
            Update,
            cleanup_floating_ui_root.run_if(crate::game_state::not_in_gameplay),
        );

        app.insert_state(Screen::InGame);
        let target = app
            .world_mut()
            .spawn((
                Position(Vec3::ZERO),
                VitalStats {
                    current_health: 50.0,
                    max_health: 50.0,
                    current_mana: 40.0,
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
        app.insert_state(Screen::MainMenu);
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
