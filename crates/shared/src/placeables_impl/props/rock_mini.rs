//! Mini decorative rock (ground scatter). Single-mesh GLB (no `_Top` node,
//! so it never participates in camera occlusion — always fully visible).

use crate::placeables::props;

#[props(
    id = "rock_mini",
    name = "Rock (Mini)",
    icon = "🪨",
    asset = "models/new/rock_mini.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.4, 0.38, 0.35),
    blocks_movement = false
)]
pub struct RockMiniProp;
