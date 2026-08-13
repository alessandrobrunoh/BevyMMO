use bevy::prelude::*;
use bevymmo_shared::abilities::AbilitySlot;

#[derive(Component)]
pub struct InscriptionWindow;

/// Toggles whether `essence_id` is the inscribed Essenza for `slot` — clicking
/// the already-active Essenza clears it (empty essence = "no essence").
#[derive(Component)]
pub struct EssenceToggleButton {
    pub slot: AbilitySlot,
    pub essence_id: String,
}

/// Toggles whether `modifier_id` is present among `slot`'s Modificatori.
#[derive(Component)]
pub struct ModifierToggleButton {
    pub slot: AbilitySlot,
    pub modifier_id: String,
}

#[derive(Component)]
pub struct CloseInscriptionButton;
