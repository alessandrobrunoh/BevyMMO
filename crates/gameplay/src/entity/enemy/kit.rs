//! When and how a kit ability is used.
//!
//! `AbilityUse` is the content-authored gate for [`super::pick::pick_ability`].
//! Targeting is stored here so a later rotation can aim without a second list.

/// Who an AI ability points at once [`super::pick::pick_ability`] has chosen it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AbilityTargeting {
    /// The combat target selected by the threat policy (tank / nearest).
    #[default]
    Main,
    /// Farthest living player from the caster.
    Farthest,
    /// Centered on the caster (self buffs, point-blank AoE).
    SelfCentered,
    /// Centroid of the `n` most tightly packed living players.
    DensestCluster { n: usize },
}

/// Extra gates on top of the ability's own cooldown and range.
///
/// `interval` is an additional wait the *caller* folds into `is_ready`
/// (ability CD **or** interval not elapsed). `pick_ability` itself only sees
/// the boolean.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AbilityUse {
    /// Extra wait besides the ability cooldown. `0` = only the ability CD.
    pub interval: f32,
    /// Skip if the combat target is closer than this. Default `0`.
    pub min_range: f32,
    /// Skip if `Some` and the combat target is farther. `None` = no max cap
    /// (the caller still range-checks the ability itself before firing).
    pub max_range: Option<f32>,
    /// Inclusive lower HP fraction. Default `0.0`.
    pub hp_above: f32,
    /// Exclusive upper HP fraction, except `1.0` (the default) which includes
    /// full health so a default kit still fires at 100% HP. Exclusive otherwise
    /// so adjacent 0.66 / 0.33 bands do not overlap.
    pub hp_below: f32,
    /// Higher wins. Tie-break is kit order (earlier first).
    pub priority: u8,
    /// Who the ability is aimed at. Default [`AbilityTargeting::Main`].
    pub targeting: AbilityTargeting,
}

impl Default for AbilityUse {
    fn default() -> Self {
        Self {
            interval: 0.0,
            min_range: 0.0,
            max_range: None,
            hp_above: 0.0,
            hp_below: 1.0,
            priority: 0,
            targeting: AbilityTargeting::Main,
        }
    }
}

impl AbilityUse {
    pub fn main() -> Self {
        Self {
            targeting: AbilityTargeting::Main,
            ..Self::default()
        }
    }

    pub fn farthest() -> Self {
        Self {
            targeting: AbilityTargeting::Farthest,
            ..Self::default()
        }
    }

    pub fn self_centered() -> Self {
        Self {
            targeting: AbilityTargeting::SelfCentered,
            ..Self::default()
        }
    }

    pub fn cluster(n: usize) -> Self {
        Self {
            targeting: AbilityTargeting::DensestCluster { n },
            ..Self::default()
        }
    }

    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn hp_above(mut self, fraction: f32) -> Self {
        self.hp_above = fraction;
        self
    }

    pub fn hp_below(mut self, fraction: f32) -> Self {
        self.hp_below = fraction;
        self
    }

    pub fn max_range(mut self, range: f32) -> Self {
        self.max_range = Some(range);
        self
    }
}

/// Whether `hp_fraction` sits in `[hp_above, hp_below)`.
///
/// `hp_below >= 1.0` is inclusive of full health so the default band (`0..=1`)
/// still matches a mob that has not been touched. Bands like `hp_below: 0.5`
/// stay exclusive on the high edge.
pub fn hp_in_band(hp_fraction: f32, hp_above: f32, hp_below: f32) -> bool {
    if hp_fraction < hp_above {
        return false;
    }
    if hp_below >= 1.0 {
        hp_fraction <= hp_below
    } else {
        hp_fraction < hp_below
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_band_includes_full_and_empty_health() {
        let use_when = AbilityUse::default();
        assert!(hp_in_band(0.0, use_when.hp_above, use_when.hp_below));
        assert!(hp_in_band(1.0, use_when.hp_above, use_when.hp_below));
        assert!(hp_in_band(0.5, use_when.hp_above, use_when.hp_below));
    }

    #[test]
    fn execute_band_is_exclusive_on_the_high_edge() {
        assert!(hp_in_band(0.49, 0.0, 0.5));
        assert!(!hp_in_band(0.5, 0.0, 0.5));
        assert!(!hp_in_band(0.66, 0.0, 0.5));
    }

    #[test]
    fn adjacent_phase_bands_do_not_overlap() {
        // Aerial [0.66, 1.0], ground [0.33, 0.66), berserk [0, 0.33).
        assert!(hp_in_band(1.0, 0.66, 1.0));
        assert!(hp_in_band(0.66, 0.66, 1.0));
        assert!(!hp_in_band(0.66, 0.33, 0.66));
        assert!(hp_in_band(0.65, 0.33, 0.66));
        assert!(!hp_in_band(0.33, 0.0, 0.33));
        assert!(hp_in_band(0.32, 0.0, 0.33));
    }
}
