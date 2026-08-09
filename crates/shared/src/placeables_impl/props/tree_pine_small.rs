//! Small pine tree with split Base/Top GLB nodes.
//!
//! The canopy node (`Tree_Pine_Small_Top`) is auto-tagged as `OccludableTop`.

use crate::placeables::props;

#[props(
    id = "tree_pine_small",
    name = "Pine Tree (Small)",
    icon = "🌲",
    asset = "models/new/tree_pine_small.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.15, 0.45, 0.2),
    blocks_movement = true,
    collision = cylinder(radius = 0.25, height = 4.0)
)]
pub struct TreePineSmallProp;
