//! Lifecycle of the game scene, driven by [`Screen`] enter/exit.

use bevy::light::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevymmo_client::local_player::LocalPlayer;

use bevymmo_network::network::protocol::Position;

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

// Camera zoom limits, measured as height above the player.
//
// These are what decide how big the character reads on screen. At 45 the whole
// 360 m map was legible but a 1.7 m character was a few pixels tall; the
// isometric MMOs this takes after sit around 16-20 m. The range still opens up
// to a wide survey view, it just no longer starts there.
const DEFAULT_CAMERA_HEIGHT: f32 = 20.0;
const MIN_CAMERA_HEIGHT: f32 = 10.0;
const MAX_CAMERA_HEIGHT: f32 = 70.0;

/// Resource to store the current camera zoom height.
#[derive(Resource, Debug, Clone)]
pub struct CameraZoom {
    pub height: f32,
}

impl Default for CameraZoom {
    fn default() -> Self {
        Self {
            height: DEFAULT_CAMERA_HEIGHT,
        }
    }
}

/// Spawns the game scene when entering [`Screen::InGame`].
///
/// Idempotent via [`GameSceneRoot`]: a duplicate OnEnter is a no-op.
pub fn spawn_game_scene(mut commands: Commands, roots: Query<Entity, With<GameSceneRoot>>) {
    if !roots.is_empty() {
        return;
    }
    spawn_game_scene_root(&mut commands);
}

/// Despawns the game scene when leaving [`Screen::InGame`]. Pause overlay
/// does not trigger this.
pub fn despawn_game_scene(mut commands: Commands, roots: Query<Entity, With<GameSceneRoot>>) {
    for root in roots.iter() {
        commands.entity(root).despawn();
    }
}

/// Moves the game camera to follow the local player (`LocalPlayer`).
///
/// By maintaining a dynamic offset based on the current zoom level relative to the local
/// player's `Position`, a rotation-free "third-person isometric" effect is achieved:
/// the camera remains fixed on the player while the server replicates movements.
/// If the local player is not yet spawned (menu/login), the camera remains
/// where scene spawn placed it.
///
/// # Example
/// ```ignore
/// // Player at (10, 0, 5) -> camera at (10, zoomed_height, zoomed_depth) looking at the player.
/// ```
pub fn follow_controlled_player(
    player: Query<(&Position, Option<&Transform>), (With<LocalPlayer>, Without<GameCamera>)>,
    mut camera: Query<&mut Transform, With<GameCamera>>,
    zoom: Res<CameraZoom>,
) {
    let Ok((player_position, player_transform)) = player.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };

    // Follow the *rendered* position, not the simulated one: `Position` only
    // moves on the fixed schedule, so anchoring the camera to it re-introduces
    // the per-tick stepping that `RenderSmoothing` exists to remove — and a
    // stuttering camera is more noticeable than a stuttering character.
    let anchor = player_transform
        .map(|transform| transform.translation)
        .unwrap_or(player_position.0);

    let camera_offset = Vec3::new(0.0, zoom.height, zoom.height);
    camera_transform.translation = anchor + camera_offset;
    camera_transform.look_at(anchor, Vec3::Y);
}

/// Handles camera zoom input from keyboard.
///
/// Allows the player to zoom in/out using the configured CameraZoomIn /
/// CameraZoomOut actions within the defined limits.
pub fn handle_camera_zoom(
    keyboard: Res<ButtonInput<KeyCode>>,
    settings: Res<bevymmo_client::user_settings::GameSettingsResource>,
    time: Res<Time>,
    mut zoom: ResMut<CameraZoom>,
) {
    use bevymmo_client::user_settings::KeyAction;

    const ZOOM_SPEED: f32 = 10.0; // Units per second

    let mut zoom_direction = 0.0;

    if settings.pressed(KeyAction::CameraZoomIn, &keyboard) {
        zoom_direction = 1.0; // Zoom in
    } else if settings.pressed(KeyAction::CameraZoomOut, &keyboard) {
        zoom_direction = -1.0; // Zoom out
    }

    if zoom_direction != 0.0 {
        let zoom_delta = zoom_direction * ZOOM_SPEED * time.delta_secs();
        zoom.height = (zoom.height - zoom_delta).clamp(MIN_CAMERA_HEIGHT, MAX_CAMERA_HEIGHT);
    }
}

fn spawn_game_scene_root(commands: &mut Commands) {
    let cam_transform = Transform::from_xyz(0.0, 15.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y);
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
                // `AmbientLight` is a per-camera override of
                // `GlobalAmbientLight`, whose default brightness of 80 left
                // everything the sun does not reach almost black. The editor
                // already uses 250; the game should not look darker than the
                // tool the map is authored in.
                AmbientLight {
                    color: Color::WHITE,
                    brightness: 250.0,
                    affects_lightmapped_meshes: true,
                },
                cam_transform,
            ));

            parent.spawn((
                Name::new("Sun Light"),
                DirectionalLight {
                    // Match the editor's sun (`editor::ground`): the Bevy
                    // default of 10 000 lux left the map noticeably flatter
                    // in game than in the tool it is authored with.
                    illuminance: 12_000.0,
                    shadow_maps_enabled: true,
                    ..default()
                },
                // The default cascade config stops casting shadows past
                // 150 m. Map 02 spans 360 m and the camera sits up to 90 m
                // above it, so the far half of the map lost its shadows at a
                // visible straight line across the terrain.
                CascadeShadowConfigBuilder {
                    maximum_distance: MAX_CAMERA_HEIGHT * 6.0,
                    first_cascade_far_bound: 40.0,
                    ..default()
                }
                .build(),
                light_transform,
            ));
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::{init_screen_states, PauseOverlay, Screen};
    use crate::renderer::RendererPlugin;
    use crate::scenes::base::BaseScenePlugin;
    use bevymmo_network::network::protocol::{EntityColor, Position};

    fn test_app() -> App {
        let mut app = App::new();
        init_screen_states(&mut app);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        // `handle_camera_zoom` reads the keyboard and the user keybinds.
        // Without these the system fails parameter validation and aborts the
        // whole schedule, taking every test in this module with it.
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<bevymmo_client::user_settings::GameSettingsResource>();
        // `handle_camera_zoom` is now also gated by `not_typing`, which reads this.
        app.init_resource::<bevymmo_client::app_state::TypingFocus>();
        app.init_resource::<Time>();
        // `sync_transforms` reads the fixed-step overstep to interpolate.
        app.init_resource::<Time<Fixed>>();
        app.add_plugins(BaseScenePlugin);
        app
    }

    fn set_screen(app: &mut App, screen: Screen) {
        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(screen);
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
            .spawn((LocalPlayer, Position(Vec3::new(10.0, 0.0, 5.0))));
        app.update();

        let mut cams = app
            .world_mut()
            .query_filtered::<&Transform, With<GameCamera>>();
        let cam = cams.single(app.world()).expect("game camera spawned");
        // Camera sits at player position + (0, height, height); the height
        // comes from `DEFAULT_CAMERA_HEIGHT`, so derive it instead of
        // hardcoding a value that goes stale when the zoom range is retuned.
        assert_eq!(
            cam.translation,
            Vec3::new(10.0, DEFAULT_CAMERA_HEIGHT, 5.0 + DEFAULT_CAMERA_HEIGHT)
        );
    }

    /// `crate::renderer::camera_view` rebuilds the camera's global transform
    /// from its local one to avoid projecting the floating UI through a
    /// frame-old camera. That shortcut is only exact while every ancestor of
    /// the camera is the identity, which today means the scene root.
    #[test]
    fn scene_root_transform_is_identity() {
        let mut app = test_app();
        set_screen(&mut app, Screen::InGame);
        app.update();

        let mut roots = app
            .world_mut()
            .query_filtered::<&Transform, With<GameSceneRoot>>();
        let root = roots.single(app.world()).expect("game scene root spawned");
        assert_eq!(
            *root,
            Transform::IDENTITY,
            "camera_view() projects through the camera's local transform; a \
             non-identity ancestor would silently offset every floating bar"
        );
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

        app.world_mut()
            .resource_mut::<NextState<PauseOverlay>>()
            .set(PauseOverlay::On);
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
