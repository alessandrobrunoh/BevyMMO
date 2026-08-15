//! Concrete `AncientWord` implementations — un file per Parola Antica,
//! mirror di `crate::spells_impl`.
//!
//! Vuoto per ora (nessuna Parola Antica ancora implementata: Eco/Dividere/
//! Invertire/Convertire/Vincolare restano contenuto futuro, per design
//! dovrebbero essere rare/scoperte in gioco — §51 del design). La funzione
//! di registrazione esiste già così il resto della pipeline (Startup,
//! `AncientWordRegistry`) è cablato e pronto quando la prima Parola verrà
//! aggiunta: nuovo file + una riga qui, come per Essenze e Modificatori.

use bevy::prelude::ResMut;

use crate::abilities::AncientWordRegistry;

pub fn register_default_ancient_words(mut _registry: ResMut<AncientWordRegistry>) {}
