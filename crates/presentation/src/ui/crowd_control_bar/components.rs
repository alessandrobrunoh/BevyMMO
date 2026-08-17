use bevy::prelude::*;

/// Root UI node that owns all crowd control screen-space bars.
#[derive(Component, Default)]
pub struct CrowdControlBarRoot;

/// Screen-space crowd control bar projected above a stunned entity.
#[derive(Component)]
pub struct ScreenCrowdControlBar {
    /// The target entity this bar follows.
    pub target_entity: Entity,
}

/// Cache of child entities and last-applied values to avoid redundant UI updates.
#[derive(Component)]
pub struct CrowdControlBarParts {
    /// The fill node entity.
    pub fill: Entity,
    /// The label text entity.
    pub label: Entity,
    /// Cached left position.
    pub last_left: Val,
    /// Cached top position.
    pub last_top: Val,
    /// Cached display state.
    pub last_display: Display,
    /// Cached fill percentage.
    pub last_fill_pct: f32,
    /// Cached label text.
    pub last_label: String,
}
