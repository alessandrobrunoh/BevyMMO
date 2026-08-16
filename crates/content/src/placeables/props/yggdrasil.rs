//! Mythical world tree. Single mesh authored at world scale (~36 m tall).
//!
//! The canopy node (`Yggdrasil_Top`) is auto-tagged as `Occludable` and fades when it blocks the camera and
//! will be hidden when it occludes the player — important given its size
//! relative to the isometric camera frustum.

use crate::placeables::props;

#[props(
    id = "yggdrasil",
    name = "Yggdrasil (World Tree)",
    icon = "🌳",
    asset = "models/new/yggdrasil.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.2, 0.4, 0.2),
    blocks_movement = true,
    collision = cylinder(radius = 1.5, height = 30.0)
)]
pub struct YggdrasilProp;
