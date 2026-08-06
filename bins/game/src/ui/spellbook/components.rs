use crate::plugins::spells::{HotbarSlot, SpellId};
use bevy::prelude::*;

#[derive(Component)]
pub struct SpellbookWindow;

#[derive(Component)]
pub struct SpellAssignmentButton {
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
pub struct CloseSpellbookButton;
