//! Reusable settings widgets.
//!
//! Each widget is a self-contained spawn function returning the root entity.
//! Marker components allow the systems in [`crate::ui::settings`] to react to
//! interactions generically, without each panel wiring its own systems.

pub mod dropdown;
pub mod key_capture;
pub mod toggle;

pub use dropdown::{
    Dropdown, DropdownChanged, DropdownItem, DropdownValueText, Select, spawn_dropdown,
    spawn_select,
};
pub use key_capture::{KeyCapture, KeyCaptureDisplay, KeyCaptureLabel, spawn_key_capture};
pub use toggle::{CheckBox, Toggle, ToggleDisplay, ToggleLabel, spawn_checkbox, spawn_toggle};
