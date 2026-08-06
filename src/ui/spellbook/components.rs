use crate::plugins::spells::{HotbarSlot, SpellId};
use bevy::prelude::*;

#[derive(Component)]
pub struct SpellbookWindow;

#[derive(Component)]
pub struct SpellListItem {
    pub spell_id: SpellId,
}

#[derive(Component)]
pub struct HotbarSlotUi {
    pub slot: HotbarSlot,
}
