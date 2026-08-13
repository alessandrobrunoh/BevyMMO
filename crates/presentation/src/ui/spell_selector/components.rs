use bevy::prelude::*;
use bevymmo_shared::spells::{HotbarSlot, SpellId};

#[derive(Component)]
pub struct SpellSelectorWindow;

/// Clicking picks `spell_id` as the active spell for `slot`. Only spawned
/// for spells actually present in that slot's `AvailableSpellChoices` — this
/// button can never represent an illegal pick.
#[derive(Component)]
pub struct SpellOptionButton {
    pub slot: HotbarSlot,
    pub spell_id: SpellId,
}

/// Text label on a [`SpellOptionButton`], refreshed every frame so the
/// currently active pick gets a checkmark without rebuilding the window.
#[derive(Component)]
pub struct SpellOptionLabel {
    pub slot: HotbarSlot,
    pub spell_id: SpellId,
}

#[derive(Component)]
pub struct ClearHotbarSlotButton {
    pub slot: HotbarSlot,
}

#[derive(Component)]
pub struct HotbarSlotLabel {
    pub slot: HotbarSlot,
}

#[derive(Component)]
pub struct CloseSpellSelectorButton;
