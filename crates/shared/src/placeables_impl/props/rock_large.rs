//! Large decorative rock. Single-mesh GLB (no `_Top` node, so it never
//! participates in camera occlusion — always fully visible).

use crate::placeables::props;

#[props(
    id = "rock_large",
    name = "Rock (Large)",
    icon = "🪨",
    asset = "models/new/rock_large.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.4, 0.38, 0.35),
    blocks_movement = true
)]
pub struct RockLargeProp;
