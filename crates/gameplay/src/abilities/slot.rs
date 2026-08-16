//! [`AbilitySlot`] — ruolo di gameplay di uno slot di abilità.
//!
//! Deliberatamente NON si chiama `Q`/`W`/`E`: il tasto fisico è un dettaglio
//! di input, rebindabile in `crate::user_settings::KeyAction`. Il legame tra
//! tasto fisico e `AbilitySlot` vive in un solo punto (il sistema di input
//! client), non nei dati di gameplay.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbilitySlot {
    Primary,
    Secondary,
    Ultimate,
}

impl AbilitySlot {
    pub const ALL: [AbilitySlot; 3] = [AbilitySlot::Primary, AbilitySlot::Secondary, AbilitySlot::Ultimate];
}
