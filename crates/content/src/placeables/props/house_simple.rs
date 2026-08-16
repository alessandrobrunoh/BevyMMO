//! Simple house shell.

use crate::placeables::props;

#[props(
    id = "house_simple",
    name = "House",
    icon = "🏠",
    scale = (3.0, 2.0, 3.0),
    tint = (0.7, 0.6, 0.4)
)]
pub struct HouseSimpleProp;
