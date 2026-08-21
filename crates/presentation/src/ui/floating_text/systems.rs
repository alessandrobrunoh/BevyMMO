//! Spawn, project, fade, and despawn floating world labels.

use bevy::prelude::*;
use bevymmo_client::server_feed::WorldTextCue;

use super::plugin::{FloatingText, SpawnFloatingText};
use crate::ui::bar::get_or_spawn_root;

/// Root UI node for all floating labels. Not parented to any 3D entity.
#[derive(Component, Default)]
pub struct FloatingTextRoot;

/// Approximate glyph width as a fraction of font size, used only to center the
/// label on the projected point before layout knows the real text width.
const CHAR_WIDTH_RATIO: f32 = 0.55;

pub fn spawn_floating_text(
    mut commands: Commands,
    mut spawns: MessageReader<SpawnFloatingText>,
    mut cues: MessageReader<WorldTextCue>,
    root_query: Query<Entity, With<FloatingTextRoot>>,
) {
    if spawns.is_empty() && cues.is_empty() {
        return;
    }

    let root = get_or_spawn_root::<FloatingTextRoot>(&mut commands, &root_query);
    commands.entity(root).insert(Pickable::IGNORE);

    for spawn in spawns.read() {
        spawn_label(&mut commands, root, spawn);
    }
    for cue in cues.read() {
        let spawn = SpawnFloatingText::from(cue);
        spawn_label(&mut commands, root, &spawn);
    }
}

fn spawn_label(commands: &mut Commands, root: Entity, spawn: &SpawnFloatingText) {
    let font_size = spawn.font_size.max(1.0);
    let estimated_width =
        (spawn.text.chars().count() as f32 * font_size * CHAR_WIDTH_RATIO).max(font_size);
    let label = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                ..default()
            },
            Text::new(spawn.text.clone()),
            TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            },
            TextColor(spawn.color),
            Pickable::IGNORE,
            FloatingText {
                base_position: spawn.world_position,
                base_color: spawn.color,
                age_seconds: 0.0,
                lifetime_seconds: spawn.lifetime_seconds.max(0.0),
                rise_speed: spawn.rise_speed,
                font_size,
                estimated_width,
            },
        ))
        .id();
    commands.entity(root).add_child(label);
}

/// Projects the risen world point through the game camera and centers the
/// label on that viewport position.
pub fn update_floating_text_position(
    camera_query: Query<(&Camera, &Transform), With<Camera3d>>,
    ui_scale: Res<UiScale>,
    mut ui_query: Query<(&FloatingText, &mut Node)>,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let camera_transform = crate::renderer::camera_view(camera_transform);
    let scale_factor = ui_scale.0;

    for (floating, mut node) in ui_query.iter_mut() {
        let world_pos =
            floating.base_position + Vec3::Y * floating.rise_speed * floating.age_seconds;
        let Ok(viewport_pos) = camera.world_to_viewport(&camera_transform, world_pos) else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        };

        let new_left = Val::Px((viewport_pos.x / scale_factor) - floating.estimated_width * 0.5);
        let new_top = Val::Px((viewport_pos.y / scale_factor) - floating.font_size * 0.5);
        if node.left != new_left {
            node.left = new_left;
        }
        if node.top != new_top {
            node.top = new_top;
        }
        if node.display != Display::Flex {
            node.display = Display::Flex;
        }
    }
}

pub fn fade_and_despawn_floating_text(
    mut commands: Commands,
    time: Res<Time>,
    mut labels: Query<(Entity, &mut FloatingText, &mut TextColor)>,
) {
    let delta = time.delta_secs();
    for (entity, mut floating, mut color) in labels.iter_mut() {
        floating.age_seconds += delta;
        if floating.age_seconds >= floating.lifetime_seconds {
            commands.entity(entity).despawn();
            continue;
        }
        let alpha = (1.0 - floating.age_seconds / floating.lifetime_seconds.max(f32::EPSILON))
            .clamp(0.0, 1.0);
        let mut faded = floating.base_color;
        faded.set_alpha(floating.base_color.alpha() * alpha);
        color.0 = faded;
    }
}

pub fn cleanup_floating_text_root(
    mut commands: Commands,
    roots: Query<Entity, With<FloatingTextRoot>>,
) {
    for root in roots.iter() {
        commands.entity(root).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::{init_screen_states, Screen};
    use crate::ui::floating_text::FloatingTextPlugin;
    use crate::ui::theme::UiTheme;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        init_screen_states(&mut app);
        app.init_resource::<UiTheme>();
        app.init_resource::<UiScale>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
        app.add_plugins(FloatingTextPlugin);
        app.insert_state(Screen::InGame);
        app
    }

    fn floating_count(world: &mut World) -> usize {
        world.query::<&FloatingText>().iter(world).count()
    }

    #[test]
    fn spawn_floating_text_creates_a_label() {
        let mut app = test_app();
        app.world_mut()
            .write_message(SpawnFloatingText::new(Vec3::ZERO, "+2 Wood"));
        app.update();
        assert_eq!(floating_count(app.world_mut()), 1);
    }

    #[test]
    fn world_text_cue_creates_a_label() {
        let mut app = test_app();
        app.world_mut()
            .write_message(WorldTextCue::new(Vec3::Y * 2.0, "+1 Wood"));
        app.update();
        assert_eq!(floating_count(app.world_mut()), 1);
    }

    #[test]
    fn label_despawns_after_lifetime_elapses() {
        let mut app = test_app();
        app.world_mut()
            .write_message(SpawnFloatingText::new(Vec3::ZERO, "+2 Wood").with_lifetime(0.12));
        app.update();
        assert_eq!(
            floating_count(app.world_mut()),
            1,
            "the label must exist until its lifetime elapses"
        );

        for _ in 0..8 {
            app.update();
        }
        assert_eq!(
            floating_count(app.world_mut()),
            0,
            "the label must despawn once age exceeds lifetime"
        );
    }
}
