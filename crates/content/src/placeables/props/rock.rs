//! Natural rock prop with a GLB model.

use crate::placeables::props;

#[props(
    id = "rock",
    name = "Rock",
    icon = "🪨",
    asset = "models/rock.glb",
    scale = (1.0, 1.0, 1.0),
    tint = (0.4, 0.38, 0.35),
    blocks_movement = true,
    collision = sphere(radius = 0.6)
)]
pub struct RockProp;
