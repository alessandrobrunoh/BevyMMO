//! Concrete built-in spell implementations.
//!
//! Each submodule is a self-contained `Spell` trait implementation with no
//! transport/rendering dependencies, so the registry in the binary (or any
//! other crate) can compose them freely.

pub mod attack;
pub mod dragon_enemy;
pub mod fireball;
pub mod healing_circle;
pub mod meteorite;
pub mod ray_of_light;
pub mod stun_field;
pub mod swift;
