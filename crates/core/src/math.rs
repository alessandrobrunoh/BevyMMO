//! Small value types the simulation needs and `glam` does not provide.
//!
//! Positions and directions are plain [`glam::Vec3`] throughout this crate —
//! deliberately, so that `bevy::math::Vec3` on the client is the *same type* and
//! nothing needs converting at the Bevy boundary. The storable spelling of a
//! vector lives with the database rows in the SpacetimeDB module, because only
//! there can it carry a `SpacetimeType` impl.

use serde::{Deserialize, Serialize};

/// A linear RGBA colour, for the few tints that are game data rather than
/// rendering detail (an essence's glyph colour, for instance).
///
/// Components are in linear space, matching `bevy_color::Rgba`, so the
/// client-side conversion is a field copy and not a colour-space change.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rgba {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl Rgba {
    pub const WHITE: Self = Self::opaque(1.0, 1.0, 1.0);

    pub const fn opaque(red: f32, green: f32, blue: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }
}

impl Default for Rgba {
    fn default() -> Self {
        Self::WHITE
    }
}
