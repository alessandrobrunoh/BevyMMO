//! Componenti della UI flottante.
//!
//! I marker (`HpBarFill`, `HpBarText`, `NameText`) sono conservati come etichette
//! semantiche anche se gli aggiornamenti usano i riferimenti diretti in
//! [`EntityBarParts`], così da evitare la scansione dei `Children` ad ogni frame.

use bevy::prelude::*;

/// Posiziona un nodo UI followando un'entità di gioco nel mondo 3D.
#[derive(Component)]
pub struct FloatingUi {
    pub target: Entity,
    pub offset: Vec3,
    /// Last computed viewport position. Used to skip relayout when unchanged.
    pub last_viewport: Option<Vec2>,
}

/// Marker: nodo riempimento della barra HP.
#[derive(Component)]
pub struct HpBarFill;

/// Marker: testo della barra HP (`current/max`).
#[derive(Component)]
pub struct HpBarText;

/// Marker: testo del nome sopra la barra.
#[derive(Component)]
pub struct NameText;

/// Riferimenti diretti alle parti figlie di una barra entità, con cache dei
/// valori già applicati a nome/fill/testo HP.
///
/// - I riferimenti diretti (`name_text`, `hp_fill`, `hp_text`) rendono gli
///   aggiornamenti O(1) senza attraversare i `Children`.
/// - I campi `last_*` consentono di saltare le scritture (`Text`, `Node`) e le
///   relative realloc/relayout quando il dato di gioco non è cambiato nel frame
///   corrente.
#[derive(Component)]
pub struct EntityBarParts {
    pub name_text: Entity,
    pub hp_fill: Entity,
    pub hp_text: Entity,
    pub last_name: String,
    pub last_hp_text: String,
    /// Percentuale fill (0.0..=100.0) già applicata al nodo `hp_fill`.
    pub last_fill_pct: f32,
}

impl EntityBarParts {
    /// Crea l'indice con cache "mai scritta": i sentinel (`-1.0` e stringhe
    /// vuote) forzano la prima scrittura anche se il dato iniziale coincide.
    pub fn new(name_text: Entity, hp_fill: Entity, hp_text: Entity) -> Self {
        Self {
            name_text,
            hp_fill,
            hp_text,
            last_name: String::new(),
            last_hp_text: String::new(),
            last_fill_pct: -1.0,
        }
    }
}
