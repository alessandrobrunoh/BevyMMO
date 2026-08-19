//! Standard `Card` UI component: reusable panel for every modular screen
//! (inventory, spellbook, character sheet, item detail, ...).
//!
//! A Card is a centered `Node` with a uniform header (title + optional close
//! button) and caller-supplied body and footer. Each card also declares an
//! exclusivity policy so that opening one panel can close another without
//! hardcoding pairs of panels.

use bevy::prelude::*;

use crate::game_state::not_typing;

pub mod builder;
pub mod components;
pub mod systems;

pub use builder::{
    ornate_bar_image, CardBuilder, CardFrameAssets, CardLayout, FRAME_INNER_PADDING,
    ORNATE_BAR_CONFIRM_PATH, ORNATE_BAR_NEUTRAL_PATH,
};
pub use components::{
    CardBody, CardExclusivityPolicy, CardFooter, CardHeader, CardKind, CardWindow, CloseCardButton,
};

/// Registers the global Card interaction systems (close button, ESC, exclusivity).
///
/// These systems run every frame on the tiny set of open cards and are
/// independent from any specific card content.
pub struct CardPlugin;

impl Plugin for CardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                systems::enforce_card_exclusivity,
                systems::close_card_on_button,
                // Reads raw `KeyCode::Escape`, not a rebindable `KeyAction` —
                // gated directly so Escape-to-leave-the-chat-field does not
                // also close whatever card happens to be open.
                systems::close_card_on_esc.run_if(not_typing),
                systems::handle_card_drag,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::ButtonInput;

    #[test]
    fn card_plugin_registers_systems_without_panicking() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>();
        // `handle_card_drag` also reads the mouse, and `UiScale` to convert
        // cursor movement into the space `Val::Px` uses. `UiPlugin` normally
        // provides the latter; this App is deliberately minimal.
        app.init_resource::<ButtonInput<MouseButton>>();
        app.init_resource::<UiScale>();
        // `close_card_on_esc` is now gated by `not_typing`, which reads this.
        app.init_resource::<bevymmo_client::app_state::TypingFocus>();
        app.add_plugins(CardPlugin);
        app.update();
    }
}
