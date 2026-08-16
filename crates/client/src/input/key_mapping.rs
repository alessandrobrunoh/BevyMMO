//! Client-only input helpers.
//!
//! Historical note: this module previously hosted a `KeyBindings` resource
//! with hardcoded keys. That resource has been superseded by
//! [`crate::user_settings::GameSettingsResource`] which is fully
//! rebindable from the Settings UI and persists across sessions. Use
//! `GameSettingsResource::just_pressed(KeyAction::X, &keys)` from now on.

pub use crate::user_settings::{GameSettingsResource, KeyAction};
