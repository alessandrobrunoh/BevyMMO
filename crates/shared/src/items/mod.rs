//! Item framework data: the `Item` trait, runtime components, effects, the
//! registry and the network commands.
//!
//! The application pipeline (processing equip/unequip requests and recomputing
//! derived stats) is server logic and lives in `bevymmo_server`. This crate
//! only defines the contract and the data, mirroring `crate::spells`.

pub mod available_spells;
pub mod components;
pub mod definition;
pub mod effects;
pub mod events;
pub mod instance;
pub mod registry;
pub mod spell_kit;

pub use available_spells::{compute_available_choices, AvailableSpellChoices};
pub use components::{EquipSlot, Equipment, Inventory, INVENTORY_CAPACITY};
pub use definition::{Item, ItemCategory, ItemConfig, ItemRarity};
pub use effects::ItemEffect;
pub use events::{EquipItemCommand, MoveItemCommand, UnequipItemCommand};
pub use instance::{ItemInstance, ItemInstanceId};
pub use registry::{ItemId, ItemRegistry};
pub use spell_kit::SpellKit;
