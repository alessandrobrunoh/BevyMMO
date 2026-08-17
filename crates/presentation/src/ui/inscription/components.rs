use bevy::prelude::*;
use bevymmo_gameplay::abilities::AbilitySlot;

#[derive(Component)]
pub struct InscriptionWindow;

/// Toggles whether `root_word_id` is the weapon's Root Word.
#[derive(Component)]
pub struct RootWordToggleButton {
    pub root_word_id: String,
}

/// Toggles whether `word_id` is present among `slot`'s secondary Ancient Words.
#[derive(Component)]
pub struct AncientWordToggleButton {
    pub slot: AbilitySlot,
    pub word_id: String,
}

/// Makes `ability_id` the active gesture on `slot`. Only spawned for slots
/// offering more than one gesture (Ultimate always offers exactly one).
#[derive(Component)]
pub struct AbilitySelectButton {
    pub slot: AbilitySlot,
    pub ability_id: String,
}

#[derive(Component)]
pub struct CloseInscriptionButton;
