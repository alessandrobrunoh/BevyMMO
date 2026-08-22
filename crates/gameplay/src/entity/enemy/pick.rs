//! Pure kit picker. The server tick and tests share this so inverted
//! range / HP comparisons cannot silently ship.

use crate::abilities::AbilityId;
use crate::entity::enemy::kit::hp_in_band;
use crate::placeables::AbilityKitEntry;

/// First ready kit entry that matches range and HP, highest `priority` wins.
///
/// Tie-break is kit order: the earlier entry is kept when priorities are equal.
///
/// `is_ready` is false when the ability is on cooldown **or** its `interval`
/// has not elapsed — the picker does not interpret those timers itself.
pub fn pick_ability(
    kit: &[AbilityKitEntry],
    distance: f32,
    hp_fraction: f32,
    is_ready: impl Fn(&AbilityId) -> bool,
) -> Option<&AbilityKitEntry> {
    let mut best: Option<&AbilityKitEntry> = None;
    for entry in kit {
        if !is_ready(&entry.ability_id) {
            continue;
        }
        if distance < entry.use_when.min_range {
            continue;
        }
        if entry
            .use_when
            .max_range
            .is_some_and(|max_range| distance > max_range)
        {
            continue;
        }
        if !hp_in_band(
            hp_fraction,
            entry.use_when.hp_above,
            entry.use_when.hp_below,
        ) {
            continue;
        }
        match best {
            Some(current) if entry.use_when.priority <= current.use_when.priority => {}
            _ => best = Some(entry),
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::inscription::KitInscription;
    use crate::entity::enemy::kit::AbilityUse;
    use std::collections::HashSet;

    const CLEAVE: &str = "cleave";
    const ROCK_THROW: &str = "rock_throw";
    const HOWL: &str = "howl";

    fn entry(id: &'static str, use_when: AbilityUse) -> AbilityKitEntry {
        AbilityKitEntry {
            ability_id: AbilityId::new(id),
            inscription: KitInscription::default(),
            use_when,
        }
    }

    fn melee_cleave() -> AbilityUse {
        AbilityUse {
            max_range: Some(5.0),
            priority: 1,
            ..AbilityUse::default()
        }
    }

    fn ranged_rock() -> AbilityUse {
        AbilityUse {
            min_range: 5.0,
            priority: 0,
            ..AbilityUse::default()
        }
    }

    fn execute_howl() -> AbilityUse {
        AbilityUse {
            hp_below: 0.5,
            priority: 10,
            ..AbilityUse::default()
        }
    }

    fn standard_kit() -> Vec<AbilityKitEntry> {
        vec![
            entry(CLEAVE, melee_cleave()),
            entry(ROCK_THROW, ranged_rock()),
            entry(HOWL, execute_howl()),
        ]
    }

    fn ready_all() -> HashSet<AbilityId> {
        [CLEAVE, ROCK_THROW, HOWL]
            .into_iter()
            .map(AbilityId::new)
            .collect()
    }

    fn pick<'a>(
        kit: &'a [AbilityKitEntry],
        distance: f32,
        hp: f32,
        ready: &HashSet<AbilityId>,
    ) -> Option<&'a str> {
        pick_ability(kit, distance, hp, |id| ready.contains(id)).map(|e| e.ability_id.as_str())
    }

    #[test]
    fn melee_distance_picks_cleave_not_rock_throw() {
        let kit = standard_kit();
        let ready = ready_all();
        assert_eq!(pick(&kit, 3.0, 1.0, &ready), Some(CLEAVE));
    }

    #[test]
    fn out_of_melee_picks_rock_throw() {
        let kit = standard_kit();
        let ready = ready_all();
        assert_eq!(pick(&kit, 10.0, 1.0, &ready), Some(ROCK_THROW));
    }

    #[test]
    fn execute_howl_beats_melee_when_hp_is_in_band() {
        let kit = standard_kit();
        let ready = ready_all();
        assert_eq!(pick(&kit, 3.0, 0.4, &ready), Some(HOWL));
    }

    #[test]
    fn howl_does_not_fire_on_the_exclusive_high_edge() {
        let kit = standard_kit();
        let ready = ready_all();
        assert_eq!(pick(&kit, 3.0, 0.5, &ready), Some(CLEAVE));
    }

    #[test]
    fn both_on_cooldown_returns_none() {
        let kit = vec![
            entry(CLEAVE, melee_cleave()),
            entry(ROCK_THROW, ranged_rock()),
        ];
        let ready = HashSet::new();
        assert_eq!(pick(&kit, 3.0, 1.0, &ready), None);
        assert_eq!(pick(&kit, 10.0, 1.0, &ready), None);
    }

    #[test]
    fn equal_priority_keeps_the_earlier_kit_entry() {
        let kit = vec![
            entry(
                CLEAVE,
                AbilityUse {
                    priority: 3,
                    ..AbilityUse::default()
                },
            ),
            entry(
                ROCK_THROW,
                AbilityUse {
                    priority: 3,
                    ..AbilityUse::default()
                },
            ),
        ];
        let ready = ready_all();
        assert_eq!(pick(&kit, 4.0, 1.0, &ready), Some(CLEAVE));
    }

    #[test]
    fn none_max_range_is_not_a_cap() {
        let kit = vec![entry(CLEAVE, AbilityUse::default())];
        let ready = ready_all();
        assert_eq!(pick(&kit, 100.0, 1.0, &ready), Some(CLEAVE));
    }

    #[test]
    fn min_range_rejects_closer_targets() {
        let kit = vec![entry(ROCK_THROW, ranged_rock())];
        let ready = ready_all();
        assert_eq!(pick(&kit, 4.99, 1.0, &ready), None);
        assert_eq!(pick(&kit, 5.0, 1.0, &ready), Some(ROCK_THROW));
    }

    #[test]
    fn max_range_rejects_farther_targets() {
        let kit = vec![entry(CLEAVE, melee_cleave())];
        let ready = ready_all();
        assert_eq!(pick(&kit, 5.0, 1.0, &ready), Some(CLEAVE));
        assert_eq!(pick(&kit, 5.01, 1.0, &ready), None);
    }

    #[test]
    fn inverted_min_range_would_fail_the_melee_case() {
        // If the gate were `distance > min_range` (skip when farther), rock
        // throw at 10 would vanish and this would pick nothing / cleave.
        let kit = standard_kit();
        let ready = ready_all();
        assert_ne!(pick(&kit, 10.0, 1.0, &ready), Some(CLEAVE));
        assert_eq!(pick(&kit, 10.0, 1.0, &ready), Some(ROCK_THROW));
    }

    #[test]
    fn inverted_max_range_would_fail_the_melee_case() {
        // If the gate were `distance < max_range` (skip when closer), cleave
        // at 3 would vanish.
        let kit = standard_kit();
        let ready = ready_all();
        assert_eq!(pick(&kit, 3.0, 1.0, &ready), Some(CLEAVE));
        assert_ne!(pick(&kit, 3.0, 1.0, &ready), Some(ROCK_THROW));
    }

    #[test]
    fn inverted_hp_below_would_skip_howl_in_execute() {
        // If the gate were `hp_fraction < hp_below` inverted to `>`, howl at
        // 0.4 would lose to cleave.
        let kit = standard_kit();
        let ready = ready_all();
        assert_eq!(pick(&kit, 3.0, 0.4, &ready), Some(HOWL));
        assert_ne!(pick(&kit, 3.0, 0.4, &ready), Some(CLEAVE));
    }

    #[test]
    fn only_the_ready_ability_is_eligible() {
        let kit = standard_kit();
        let ready = [ROCK_THROW]
            .into_iter()
            .map(AbilityId::new)
            .collect::<HashSet<_>>();
        assert_eq!(pick(&kit, 3.0, 1.0, &ready), None);
        assert_eq!(pick(&kit, 10.0, 1.0, &ready), Some(ROCK_THROW));
    }
}
