//! Sistema di targeting con tasto destro e ray-sphere intersection.
//!
//! Fornisce:
//! - [`CurrentTarget`] resource per tenere traccia del target selezionato
//! - Sistema di picking geometrico semplice senza dipendenze da collider
//! - Auto-pulizia del target quando l'entità sparisce o perde componenti richiesti

mod plugin;
mod resources;
mod systems;

pub use plugin::TargetingPlugin;
pub use resources::CurrentTarget;
