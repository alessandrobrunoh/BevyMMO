//! Local UI components for the boss bar.

use bevy::prelude::*;

/// Root node of the boss bar overlay (the HP bar + name). Its visibility is
/// toggled by the update system based on whether an engaged, living boss exists.
#[derive(Component)]
pub struct BossBarRoot;

/// The fill sprite of the HP bar; its width is scaled to the HP fraction.
#[derive(Component)]
pub struct BossBarFill;

/// Root of the transient phase banner; despawned after the banner timer elapses.
#[derive(Component)]
pub struct BossBanner;

/// Tracks the last observed phase so the update system can detect transitions
/// and arm the banner. Lives in a resource because there is at most one boss.
#[derive(Resource, Default)]
pub struct BossBannerState {
    pub last_phase: Option<bevymmo_gameplay::entity::boss::components::BossPhase>,
    pub remaining_seconds: f32,
}
