//! Reusable settings widgets.
//!
//! Each widget is a self-contained spawn function returning the root entity.
//! Marker components allow the systems in [`crate::ui::settings`] to react to
//! interactions generically, without each panel wiring its own systems.

pub mod dropdown;
pub mod key_capture;
pub mod toggle;

pub use dropdown::{
    spawn_dropdown, spawn_select, Dropdown, DropdownChanged, DropdownItem, DropdownValueText,
    Select,
};
pub use key_capture::{spawn_key_capture, KeyCapture, KeyCaptureDisplay, KeyCaptureLabel};
pub use toggle::{spawn_checkbox, spawn_toggle, CheckBox, Toggle, ToggleDisplay, ToggleLabel};
