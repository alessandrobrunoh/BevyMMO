//! Boss entity — Vermithrax, the Ashen Drake (dragon encounter).
//!
//! Mirrors the `enemy` plugin layout. Phase 0 registers only the replicated
//! components and spawns a dormant, immobile dragon. Aggro, threat, the phase
//! machine and the ability rotation are added in later phases.

#[cfg(feature = "client")]
pub mod arena_visual;
pub mod components;
#[cfg(feature = "client")]
pub mod dragon_visual;
pub mod spawn;
pub mod systems;
pub mod target_select;

use bevy::prelude::*;

// Imported for `register_type` registration of replicated components.
use components::{BossArena, BossPhase};

// Only the marker is re-exported at module root for now. ThreatTable,
// BossSpellbook and BossRotationState are consumed by server systems added
// in Phase 1+; they are re-exported then to avoid unused-import warnings at
// every phase boundary.
pub use components::Boss;

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        // Replicated components must be registered in the Bevy type registry
        // for lightyear to (de)serialize them.
        app.register_type::<BossPhase>();
        app.register_type::<BossArena>();

        // Server-authoritative encounter control. `accrue_threat` runs before
        // the rotation driver (Phase 2) so a fresh damage tick can influence
        // the next target pick within the same fixed step.
        app.add_systems(
            FixedUpdate,
            (
                systems::boss_aggro_check,
                systems::accrue_threat,
                systems::update_boss_phase,
                systems::boss_chase,
                systems::run_boss_rotation,
            )
                .chain()
                .run_if(crate::network::mode::has_server),
        );

        #[cfg(feature = "client")]
        arena_visual::client_arena_systems(app);

        #[cfg(feature = "client")]
        dragon_visual::client_dragon_systems(app);
    }
}
