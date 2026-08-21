//! Registration and the spawn request for world-space floating labels.

use super::systems;
use bevy::prelude::*;
use bevymmo_client::server_feed::WorldTextCue;

use crate::game_state::{Screen, not_in_gameplay};
use crate::renderer::RenderSync;

const DEFAULT_LIFETIME_SECONDS: f32 = 1.5;
const DEFAULT_RISE_SPEED: f32 = 1.25;
const DEFAULT_FONT_SIZE: f32 = 18.0;

/// Request to spawn a floating label at a world position.
/// Presentation consumes this; other crates send it.
#[derive(Message, Clone, Debug)]
pub struct SpawnFloatingText {
    pub world_position: Vec3,
    pub text: String,
    pub color: Color,
    pub lifetime_seconds: f32,
    pub rise_speed: f32, // world-units/sec upward
    pub font_size: f32,
}

impl SpawnFloatingText {
    pub fn new(world_position: Vec3, text: impl Into<String>) -> Self {
        Self {
            world_position,
            text: text.into(),
            color: Color::WHITE,
            lifetime_seconds: DEFAULT_LIFETIME_SECONDS,
            rise_speed: DEFAULT_RISE_SPEED,
            font_size: DEFAULT_FONT_SIZE,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_lifetime(mut self, seconds: f32) -> Self {
        self.lifetime_seconds = seconds;
        self
    }

    pub fn with_rise_speed(mut self, speed: f32) -> Self {
        self.rise_speed = speed;
        self
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }
}

impl From<&WorldTextCue> for SpawnFloatingText {
    fn from(cue: &WorldTextCue) -> Self {
        Self {
            world_position: cue.world_position,
            text: cue.text.clone(),
            color: cue.color,
            lifetime_seconds: cue.lifetime_seconds,
            rise_speed: cue.rise_speed,
            font_size: cue.font_size,
        }
    }
}

/// One floating label. Lives on a screen-space UI node, not on a 3D entity.
#[derive(Component)]
pub struct FloatingText {
    pub(super) base_position: Vec3,
    pub(super) base_color: Color,
    pub(super) age_seconds: f32,
    pub(super) lifetime_seconds: f32,
    pub(super) rise_speed: f32,
    pub(super) font_size: f32,
    pub(super) estimated_width: f32,
}

pub struct FloatingTextPlugin;

impl Plugin for FloatingTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnFloatingText>();
        app.add_message::<WorldTextCue>();
        app.add_systems(
            Update,
            (
                systems::spawn_floating_text,
                (
                    systems::update_floating_text_position.in_set(RenderSync::Project),
                    systems::fade_and_despawn_floating_text,
                )
                    .chain(),
            )
                .chain()
                .run_if(in_state(Screen::InGame)),
        )
        .add_systems(
            Update,
            systems::cleanup_floating_text_root.run_if(not_in_gameplay),
        );
    }
}
