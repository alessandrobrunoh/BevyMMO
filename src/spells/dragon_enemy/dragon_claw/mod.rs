//! `dragon_claw` — the dragon's instant melee filler.
//!
//! Single-target damage on the boss's current threat target, used to keep melee
//! pressure and generate threat cadence between the AoE abilities. It reads
//! `ctx.target_entity` (resolved by the boss rotation from the threat table).

pub mod definition;
#[cfg(feature = "client")]
pub mod visual;

pub use definition::DragonClawSpell;
