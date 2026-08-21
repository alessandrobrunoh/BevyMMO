//! UI flottante che visualizza nome e punti vita di un'entità.

pub mod components;
mod plugin;
pub mod systems;

pub use components::{EntityBarParts, FloatingUi, HpBarFill, HpBarText, NameText};
pub use plugin::{spawn_entity_bar, EntityBarPlugin};
