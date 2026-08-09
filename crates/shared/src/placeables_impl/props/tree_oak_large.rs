//! Large broadleaf oak tree with split Base/Top GLB nodes.
//!
//! The canopy node (`Tree_Oak_Large_Top`) is auto-tagged as `OccludableTop`
//! by the presentation layer so it fades when blocking the camera's view
//! of the player.

use crate::placeables::props;

#[props(
    id = "tree_oak_large",
    name = "Oak Tree (Large)",
    icon = "🌳",
    asset = "models/new/tree_oak_large.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.2, 0.5, 0.2),
    blocks_movement = true
)]
pub struct TreeOakLargeProp;
