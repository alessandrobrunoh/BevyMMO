//! Medium birch tree with split Base/Top GLB nodes.
//!
//! The canopy node (`Tree_Birch_Medium_Top`) is auto-tagged as `Occludable` and fades when it blocks the camera.

use crate::placeables::props;

#[props(
    id = "tree_birch_medium",
    name = "Birch Tree (Medium)",
    icon = "🌳",
    asset = "models/new/tree_birch_medium.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.3, 0.5, 0.25),
    blocks_movement = true,
    collision = cylinder(radius = 0.3, height = 5.0)
)]
pub struct TreeBirchMediumProp;
