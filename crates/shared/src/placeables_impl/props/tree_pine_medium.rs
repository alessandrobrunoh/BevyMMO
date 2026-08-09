//! Medium pine tree with split Base/Top GLB nodes.
//!
//! The canopy node (`Tree_Pine_Medium_Top`) is auto-tagged as `OccludableTop`.

use crate::placeables::props;

#[props(
    id = "tree_pine_medium",
    name = "Pine Tree (Medium)",
    icon = "🌲",
    asset = "models/new/tree_pine_medium.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.15, 0.45, 0.2),
    blocks_movement = true
)]
pub struct TreePineMediumProp;
