//! Concrete `AncientWord` implementations — un file per Parola Antica,
//! mirror di `crate::spells_impl`.
//!
//! Vuoto per ora (nessuna Parola Antica ancora implementata: Eco/Dividere/
//! Invertire/Convertire/Vincolare restano contenuto futuro, per design
//! dovrebbero essere rare/scoperte in gioco — §51 del design). La funzione
//! di registrazione esiste già così il resto della pipeline (Startup,
//! `AncientWordRegistry`) è cablato e pronto quando la prima Parola verrà
//! aggiunta: nuovo file + una riga qui, come per Essenze e Modificatori.


use crate::abilities::AncientWordRegistry;

/// Builds the registry containing every entry this build ships.
///
/// Returns the registry rather than filling a Bevy `Resource`: the
/// SpacetimeDB module has no `Startup` schedule and no ECS to put one in.
/// `bevymmo_shared` wraps this in a system for the client.
pub fn default_ancient_words() -> AncientWordRegistry {
    #[allow(unused_mut)]
    let mut registry = AncientWordRegistry::default();
    registry
}
