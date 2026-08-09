//! Lifecycle of the game scene, driven by [`GameScreen`].

use bevy::prelude::*;
use lightyear::prelude::Controlled;

use bevymmo_shared::network::protocol::Position;

use crate::game_state::{GameScreen, Screen};

/// Marker for the game scene root (camera, light, ground).
///
/// Only one entity exists with this component per client: the lifecycle
/// system uses it both to prevent duplicate spawns and for cleanup.
#[derive(Component, Debug, Clone, Copy)]
pub struct GameSceneRoot;

/// Marker for the 3D camera of the game scene.
///
/// Used by the follow system to uniquely identify it (the client also has
/// a `Camera2d` for the UI and one or more `Camera3d` for debug/testing).
#[derive(Component, Debug, Clone, Copy)]
pub struct GameCamera;

/// Constant camera offset relative to the followed player.
///
/// Maintains the same isometric framing from spawn even as the player
/// moves: 25 units high and 25 in depth relative to the target.
const CAMERA_OFFSET: Vec3 = Vec3::new(0.0, 25.0, 25.0);

/// Spawns/despawns the game scene based on [`GameScreen`].
///
/// - `InGame`/`Paused` + no root: spawns the scene.
/// - `MainMenu`/`Settings`/`Connecting` + root present: recursive despawn.
///
/// The system is idempotent: it can run every frame without side effects
/// when the state doesn't change.
pub fn update_game_scene_lifecycle(
    mut commands: Commands,
    screen: Res<GameScreen>,
    roots: Query<Entity, With<GameSceneRoot>>,
) {
    let in_game = matches!(screen.0, Screen::InGame | Screen::Paused);
    let has_root = roots.iter().next().is_some();

    if in_game && !has_root {
        spawn_game_scene(&mut commands);
    } else if !in_game && has_root {
        for root in roots.iter() {
            // recursive despawn: removes camera and light.
            commands.entity(root).despawn();
        }
    }
}

/// Moves the game camera to follow the local player (`Controlled`).
///
/// By maintaining a constant offset ([`CAMERA_OFFSET`]) relative to the local
/// player's `Position`, a rotation-free "third-person isometric" effect is achieved:
/// the camera remains fixed on the player while the server replicates movements.
/// If the local player is not yet spawned (menu/login), the camera remains
/// where scene spawn placed it.
///
/// # Example
/// ```ignore
/// // Player at (10, 0, 5) -> camera at (10, 25, 30) looking at the player.
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

fn spawn_game_scene(commands: &mut Commands) {
    let cam_transform = Transform::from_xyz(0.0, 25.0, 25.0).looking_at(Vec3::ZERO, Vec3::Y);
    let light_transform = Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y);

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
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::RendererPlugin;
    use crate::scenes::base::BaseScenePlugin;
    use bevymmo_shared::network::protocol::{EntityColor, Position};

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

        // Local player controlled by the client.
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
        assert_eq!(before, after, "no controlled player -> stationary camera");
    }

    #[test]
    fn base_scene_is_not_spawned_in_main_menu() {
        let mut app = test_app();
        set_screen(&mut app, Screen::MainMenu);
        app.update();
        assert_eq!(root_count(&mut app), 0, "no scene in menu");
    }

    #[test]
    fn entering_ingame_spawns_exactly_one_root() {
        let mut app = test_app();
        set_screen(&mut app, Screen::InGame);
        app.update();
        assert_eq!(root_count(&mut app), 1);
        // idempotent: a second update does not duplicate.
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
        assert_eq!(root_count(&mut app), 1, "Paused is an overlay, not despawn");
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
            assert_eq!(root_count(&mut app), 1, "re-entry failed for {:?}", target);

            set_screen(&mut app, target);
            app.update();
            assert_eq!(root_count(&mut app), 0, "cleanup failed for {:?}", target);
        }
    }

    #[test]
    fn renderer_strips_local_render_components_when_leaving_game() {
        let mut app = test_app();
        app.add_plugins(RendererPlugin);

        // Replicated game entity: has Position/EntityColor but no rendering.
        let entity = app
            .world_mut()
            .spawn((Position(Vec3::ZERO), EntityColor(Color::BLACK)))
            .id();

        // InGame: renderer adds Mesh3d/MeshMaterial3d/Transform.
        set_screen(&mut app, Screen::InGame);
        app.update();
        app.update();
        let world = app.world();
        assert!(
            world.entity(entity).get::<Mesh3d>().is_some(),
            "renderer should have spawned mesh InGame"
        );

        // Return to menu: local render components are removed but
        // Position/EntityColor remain (replicated, not local).
        set_screen(&mut app, Screen::MainMenu);
        app.update();
        let world = app.world();
        let entity_ref = world.entity(entity);
        assert!(
            entity_ref.get::<Mesh3d>().is_none(),
            "Mesh3d should disappear"
        );
        assert!(
            entity_ref
                .get::<MeshMaterial3d<StandardMaterial>>()
                .is_none(),
            "MeshMaterial3d should disappear"
        );
        assert!(
            entity_ref.get::<Transform>().is_none(),
            "Transform should disappear"
        );
        assert!(
            entity_ref.get::<Position>().is_some(),
            "Position is replicated"
        );
        assert!(
            entity_ref.get::<EntityColor>().is_some(),
            "EntityColor is replicated"
        );

        // Re-entry: renderer recreates render components.
        set_screen(&mut app, Screen::InGame);
        app.update();
        app.update();
        assert!(
            app.world().entity(entity).get::<Mesh3d>().is_some(),
            "renderer should recreate mesh on re-entry"
        );
    }
}
