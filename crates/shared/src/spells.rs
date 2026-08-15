//! Spell data and the cast contract — re-exported from [`bevymmo_domain::spells`].
//!
//! Moved to the domain crate so the SpacetimeDB module can run `Spell::cast`
//! itself: the cast context is a plain buffer of pending effects, so the reducer
//! drains exactly what the Bevy systems used to drain.
pub use bevymmo_domain::spells::*;
