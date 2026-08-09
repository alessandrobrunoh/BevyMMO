//! Medium broadleaf oak tree with split Base/Top GLB nodes.
//!
//! The canopy node (`Tree_Oak_Medium_Top`) is auto-tagged as `OccludableTop`.

use crate::placeables::props;

#[props(
    id = "tree_oak_medium",
    name = "Oak Tree (Medium)",
    icon = "🌳",
    asset = "models/new/tree_oak_medium.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.2, 0.5, 0.2),
    blocks_movement = true,
    collision = cylinder(radius = 0.4, height = 5.5)
)]
pub struct TreeOakMediumProp;
