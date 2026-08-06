//! Plugin per il target frame (UI panel con info sul target selezionato).

use crate::ui::systems::in_gameplay;
use bevy::prelude::*;

use super::systems::{cleanup_target_frames, manage_target_frame, update_target_frame_content};

pub struct TargetFramePlugin;

impl Plugin for TargetFramePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                manage_target_frame,
                (update_target_frame_content, cleanup_target_frames).chain(),
            )
                .chain()
                .run_if(in_gameplay),
        );
    }
}
