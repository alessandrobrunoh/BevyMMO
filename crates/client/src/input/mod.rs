//! Client-only input helpers.
//!
//! The historical `KeyBindings` resource has been superseded by
//! [`bevymmo_shared::user_settings::GameSettingsResource`], which is fully
//! rebindable from the Settings UI and persists across sessions.

pub mod key_mapping;

pub use key_mapping::{GameSettingsResource, KeyAction};
