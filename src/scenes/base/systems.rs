//! Lifecycle della scena di gioco, guidato da [`GameScreen`].

use bevy::color::Color;
use bevy::prelude::*;
use lightyear::prelude::Controlled;

use crate::game_state::{GameScreen, Screen};
use crate::network::protocol::Position;

/// Marker per la root della scena di gioco (camera, luce, terreno).
///
/// Esiste una sola entità con questo componente per client: il sistema di
/// lifecycle lo usa sia per evitare spawn duplicati sia per il cleanup.
#[derive(Component, Debug, Clone, Copy)]
pub struct GameSceneRoot;

/// Marker della camera 3D della scena di gioco.
///
/// Serve al sistema di follow per individuarla in modo univoco (il client ha
/// anche una `Camera2d` per la UI e una o piu' `Camera3d` di debug/test).
#[derive(Component, Debug, Clone, Copy)]
pub struct GameCamera;

/// Offset costante della camera rispetto al player seguito.
///
/// Mantiene la stessa inquadratura isometrica di spawn anche mentre il player
/// si muove: 25 unita' in altezza e 25 in profondita' rispetto al target.
const CAMERA_OFFSET: Vec3 = Vec3::new(0.0, 25.0, 25.0);

/// Spawn/despawn la scena di gioco in base a [`GameScreen`].
///
/// - `InGame`/`Paused` + nessuna root: spawna la scena.
/// - `MainMenu`/`Settings`/`Connecting` + root presente: despawn ricorsivo.
///
/// Il sistema è idempotente: può girare ogni frame senza effetti collaterali
/// quando lo stato non cambia.
pub fn update_game_scene_lifecycle(
    mut commands: Commands,
    screen: Res<GameScreen>,
    roots: Query<Entity, With<GameSceneRoot>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let in_game = matches!(screen.0, Screen::InGame | Screen::Paused);
    let has_root = roots.iter().next().is_some();

    if in_game && !has_root {
        spawn_game_scene(&mut commands, &mut meshes, &mut materials);
    } else if !in_game && has_root {
        for root in roots.iter() {
            // despawn ricorsivo: rimuove camera, luce e terreno.
            commands.entity(root).despawn();
        }
    }
}

/// Sposta la camera di gioco per seguire il player locale (`Controlled`).
///
/// Mantenendo un offset costante ([`CAMERA_OFFSET`]) rispetto alla `Position`
/// del player locale si ottiene un effetto "third-person isometrico" senza
/// rotazioni: la camera rimane fissa sul player mentre il server replica i
/// movimenti. Se il player locale non e' ancora spawnato (menu/login) la
/// camera resta dove l'ha messa lo spawn della scena.
///
/// # Esempio
/// ```ignore
/// // Player in (10, 0, 5) -> camera in (10, 25, 30) rivolta verso il player.
/// ```
pub fn follow_controlled_player(
    player: Query<&Position, With<Controlled>>,
    mut camera: Query<&mut Transform, With<GameCamera>>,
) {
    let Ok(player_position) = player.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };

    let target = player_position.0 + CAMERA_OFFSET;
    let look_at = player_position.0;
    camera_transform.translation = target;
    camera_transform.look_at(look_at, Vec3::Y);
}

fn spawn_game_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let cam_transform = Transform::from_xyz(0.0, 25.0, 25.0).looking_at(Vec3::ZERO, Vec3::Y);
    let light_transform = Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y);

    let plane_mesh = meshes.add(Plane3d::default().mesh().size(50.0, 50.0));
    let plane_mat = materials.add(Color::srgb(0.2, 0.2, 0.2));

    commands
        .spawn((
            Name::new("Game Scene Root"),
            GameSceneRoot,
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Game Camera"),
                GameCamera,
                Camera3d::default(),
                cam_transform,
            ));

            parent.spawn((
                Name::new("Sun Light"),
                DirectionalLight {
                    shadow_maps_enabled: true,
                    ..default()
                },
                light_transform,
            ));

            parent.spawn((
                Name::new("Ground"),
                Mesh3d(plane_mesh),
                MeshMaterial3d::<StandardMaterial>(plane_mat),
            ));
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::protocol::{EntityColor, Position};
    use crate::plugins::renderer::RendererPlugin;
    use crate::scenes::base::BaseScenePlugin;

    fn test_app() -> App {
        let mut app = App::new();
        app.init_resource::<GameScreen>();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.add_plugins(BaseScenePlugin);
        app
    }

    fn set_screen(app: &mut App, screen: Screen) {
        app.world_mut().resource_mut::<GameScreen>().0 = screen;
    }

    fn root_count(app: &mut App) -> usize {
        app.world_mut()
            .query::<&GameSceneRoot>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn camera_follows_controlled_player_position() {
        let mut app = test_app();
        set_screen(&mut app, Screen::InGame);
        app.update();

        // Player locale controllato dal client.
        app.world_mut()
            .spawn((Controlled, Position(Vec3::new(10.0, 0.0, 5.0))));
        app.update();

        let mut cams = app
            .world_mut()
            .query_filtered::<&Transform, With<GameCamera>>();
        let cam = cams.single(app.world()).expect("game camera spawned");
        assert_eq!(cam.translation, Vec3::new(10.0, 25.0, 30.0));
    }

    #[test]
    fn camera_stays_put_without_controlled_player() {
        let mut app = test_app();
        set_screen(&mut app, Screen::InGame);
        app.update();

        let before = app
            .world_mut()
            .query_filtered::<&Transform, With<GameCamera>>()
            .single(app.world())
            .expect("game camera spawned")
            .translation;
        app.update();
        let after = app
            .world_mut()
            .query_filtered::<&Transform, With<GameCamera>>()
            .single(app.world())
            .expect("game camera spawned")
            .translation;
        assert_eq!(before, after, "nessun player controllato -> camera ferma");
    }

    #[test]
    fn base_scene_is_not_spawned_in_main_menu() {
        let mut app = test_app();
        set_screen(&mut app, Screen::MainMenu);
        app.update();
        assert_eq!(root_count(&mut app), 0, "nessuna scena nel menu");
    }

    #[test]
    fn entering_ingame_spawns_exactly_one_root() {
        let mut app = test_app();
        set_screen(&mut app, Screen::InGame);
        app.update();
        assert_eq!(root_count(&mut app), 1);
        // idempotente: un secondo update non duplica.
        app.update();
        assert_eq!(root_count(&mut app), 1);
    }

    #[test]
    fn paused_keeps_the_scene_alive() {
        let mut app = test_app();
        set_screen(&mut app, Screen::InGame);
        app.update();
        assert_eq!(root_count(&mut app), 1);

        set_screen(&mut app, Screen::Paused);
        app.update();
        assert_eq!(root_count(&mut app), 1, "Paused e' un overlay, non despawn");
    }

    #[test]
    fn returning_to_menu_despawns_scene() {
        let mut app = test_app();
        set_screen(&mut app, Screen::InGame);
        app.update();
        assert_eq!(root_count(&mut app), 1);

        for target in [Screen::MainMenu, Screen::Settings, Screen::Connecting] {
            set_screen(&mut app, Screen::InGame);
            app.update();
            assert_eq!(root_count(&mut app), 1, "re-entry fallita per {:?}", target);

            set_screen(&mut app, target);
            app.update();
            assert_eq!(root_count(&mut app), 0, "cleanup fallito per {:?}", target);
        }
    }

    #[test]
    fn renderer_strips_local_render_components_when_leaving_game() {
        let mut app = test_app();
        app.add_plugins(RendererPlugin);

        // Entità di gioco replicata: ha Position/EntityColor ma niente render.
        let entity = app
            .world_mut()
            .spawn((Position(Vec3::ZERO), EntityColor(Color::BLACK)))
            .id();

        // InGame: il renderer aggiunge Mesh3d/MeshMaterial3d/Transform.
        set_screen(&mut app, Screen::InGame);
        app.update();
        app.update();
        let world = app.world();
        assert!(
            world.entity(entity).get::<Mesh3d>().is_some(),
            "il renderer doveva aver spawnato la mesh InGame"
        );

        // Torna al menu: i componenti render locali vengono rimossi ma
        // Position/EntityColor restano (sono replicati, non locali).
        set_screen(&mut app, Screen::MainMenu);
        app.update();
        let world = app.world();
        let entity_ref = world.entity(entity);
        assert!(entity_ref.get::<Mesh3d>().is_none(), "Mesh3d deve sparire");
        assert!(
            entity_ref
                .get::<MeshMaterial3d<StandardMaterial>>()
                .is_none(),
            "MeshMaterial3d deve sparire"
        );
        assert!(
            entity_ref.get::<Transform>().is_none(),
            "Transform deve sparire"
        );
        assert!(
            entity_ref.get::<Position>().is_some(),
            "Position e' replicata"
        );
        assert!(
            entity_ref.get::<EntityColor>().is_some(),
            "EntityColor e' replicata"
        );

        // Re-entry: il renderer ricrea i componenti render.
        set_screen(&mut app, Screen::InGame);
        app.update();
        app.update();
        assert!(
            app.world().entity(entity).get::<Mesh3d>().is_some(),
            "il renderer deve ricreare la mesh al re-entry"
        );
    }
}
