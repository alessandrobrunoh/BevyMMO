//! Large flat boulder. Single-mesh GLB (no `_Top` node, so it never
//! participates in camera occlusion — always fully visible).

use crate::placeables::props;

#[props(
    id = "boulder_large",
    name = "Boulder (Large)",
    icon = "🪨",
    asset = "models/new/boulder_large.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.42, 0.4, 0.36),
    blocks_movement = true,
    collision = box(2.4, 1.2, 2.0)
)]
pub struct BoulderLargeProp;
