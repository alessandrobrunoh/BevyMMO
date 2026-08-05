//! UI flottante che visualizza nome e punti vita di un'entità.

pub mod components;
pub mod systems;
mod plugin;

pub use plugin::{spawn_entity_bar, EntityBarPlugin};
pub use components::{EntityBarParts, FloatingUi, HpBarFill, HpBarText, NameText};
