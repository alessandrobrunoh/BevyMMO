//! Medium decorative rock. Single-mesh GLB (no `_Top` node, so it never
//! participates in camera occlusion — always fully visible).

use crate::placeables::props;

#[props(
    id = "rock_medium",
    name = "Rock (Medium)",
    icon = "🪨",
    asset = "models/new/rock_medium.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.4, 0.38, 0.35),
    blocks_movement = true
)]
pub struct RockMediumProp;
