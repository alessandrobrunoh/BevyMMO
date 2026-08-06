//! Boss AI systems (aggro, threat accrual, phase machine, ability rotation).
//!
//! All systems here run server-side only (gated by `has_server`) once
//! implemented. Phase 0 ships the boss without AI: it spawns dormant and
//! immobile.
