//! Broadleaf oak tree with a GLB model.

use crate::placeables::props;

#[props(
    id = "tree_oak",
    name = "Oak Tree",
    icon = "🌳",
    asset = "models/tree_oak.glb",
    scale = (0.8, 2.5, 0.8),
    tint = (0.2, 0.5, 0.2)
)]
pub struct TreeOakProp;
