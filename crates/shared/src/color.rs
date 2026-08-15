//! Bridging the domain's colour type to Bevy's.
//!
//! `bevymmo_domain::Rgba` exists because depending on `bevy_color` would drag
//! `bevy_reflect`, `bevy_math` and `bevy_platform` into a crate whose whole
//! point is not to depend on Bevy. Neither type belongs to this crate, so the
//! conversion is a free function rather than a `From` impl.

use bevy::color::{Color, LinearRgba};
use bevymmo_domain::Rgba;

/// The domain colour as Bevy's linear RGBA. Both are linear, so this is a
/// field copy, not a colour-space conversion.
pub fn to_linear(colour: Rgba) -> LinearRgba {
    LinearRgba::new(colour.red, colour.green, colour.blue, colour.alpha)
}

/// The domain colour as a Bevy `Color`.
pub fn to_color(colour: Rgba) -> Color {
    Color::LinearRgba(to_linear(colour))
}
