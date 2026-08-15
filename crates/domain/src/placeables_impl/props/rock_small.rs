//! Small decorative rock. Single-mesh GLB (no `_Top` node, so it never
//! participates in camera occlusion — always fully visible).

use crate::placeables::props;

#[props(
    id = "rock_small",
    name = "Rock (Small)",
    icon = "🪨",
    asset = "models/new/rock_small.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.4, 0.38, 0.35),
    blocks_movement = true,
    collision = sphere(radius = 0.4)
)]
pub struct RockSmallProp;
