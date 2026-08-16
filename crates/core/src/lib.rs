//! Stable, engine-independent primitive types shared by every game layer.

pub mod ids;
pub mod math;

pub use ids::{EntityId, KindId};
pub use math::Rgba;
