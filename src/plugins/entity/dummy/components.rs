//! Componenti specifici del Dummy.

use bevy::prelude::*;

/// Marker component per identificare un'entità Dummy.
///
/// Il Dummy è un bersaglio statico con HP enormi, usato per testare
/// sistema danni, UI targeting e spell. Non ha AI, non si muove e non
/// ha spellbook.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Dummy;
