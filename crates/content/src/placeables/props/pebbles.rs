//! Loose pebbles scatter prop. Decorative only — does not block movement.

use crate::placeables::props;

#[props(
    id = "pebbles",
    name = "Pebbles",
    icon = "🪨",
    asset = "models/new/pebbles.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.5, 0.48, 0.45),
    blocks_movement = false
)]
pub struct PebblesProp;
