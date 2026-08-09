//! Large pine tree with split Base/Top GLB nodes.
//!
//! The canopy node (`Tree_Pine_Large_Top`) is auto-tagged as `OccludableTop`.

use crate::placeables::props;

#[props(
    id = "tree_pine_large",
    name = "Pine Tree (Large)",
    icon = "🌲",
    asset = "models/new/tree_pine_large.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.15, 0.45, 0.2),
    blocks_movement = true,
    collision = cylinder(radius = 0.45, height = 7.0)
)]
pub struct TreePineLargeProp;
