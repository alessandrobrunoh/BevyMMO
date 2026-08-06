//! Boss entity — Vermithrax, the Ashen Drake (dragon encounter).
//!
//! Mirrors the `enemy` plugin layout. Phase 0 registers only the replicated
//! components and spawns a dormant, immobile dragon. Aggro, threat, the phase
//! machine and the ability rotation are added in later phases.

pub mod components;
pub mod spawn;
pub mod systems;

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
        // Phase 0: no AI systems yet. Aggro/threat/phase/rotation arrive in
        // Phase 1+, all gated by `crate::network::mode::has_server`.
    }
}
