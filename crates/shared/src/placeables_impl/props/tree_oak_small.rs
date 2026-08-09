//! Small broadleaf oak tree with split Base/Top GLB nodes.
//!
//! The canopy node (`Tree_Oak_Small_Top`) is auto-tagged as `OccludableTop`.

use crate::placeables::props;

#[props(
    id = "tree_oak_small",
    name = "Oak Tree (Small)",
    icon = "🌳",
    asset = "models/new/tree_oak_small.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.2, 0.5, 0.2),
    blocks_movement = true
)]
pub struct TreeOakSmallProp;
