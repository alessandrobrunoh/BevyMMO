//! Definition contract of a game entity.
//!
//! `EntityDefinition` is NOT an OOP "is-a" trait: it is a *data* contract
//! that each concrete entity implements to centralize spawning and network
//! configuration. In Bevy it is idiomatic to have marker components
//! + data bundles, not runtime polymorphism. This trait declares
//! "how an entity is built" and which network components it requires.

use bevy::color::Color;
use bevy::prelude::*;
use lightyear::prelude::NetworkTarget;

use crate::entity::components::EntityKind;
use crate::stats::components::StatsBundleData;

/// Every game entity implements this trait. The `spawn_entity::<T>()` helper
/// uses it to construct the entity uniformly, automatically applying
/// `GameEntity`, stat components (`MovementStats`, `CombatStats`, `VitalStats`),
/// `Position`, `EntityColor`, and lightyear replication. Thus every new entity
/// is automatically synchronized over the network without manual configuration.
pub trait EntityDefinition: Component {
    /// Readable name (logging, debug).
    fn name() -> &'static str;

    /// Bundle of specific identity/data components (only markers + components
    /// belonging to this entity, NOT `Position`/`EntityColor`/stats which are
    /// managed by the central spawn system).
    fn bundle() -> impl Bundle;

    /// Initial position. Default `Vec3::ZERO`.
    fn initial_position() -> Vec3 {
        Vec3::ZERO
    }

    /// Initial color. Default neutral gray.
    fn initial_color() -> Color {
        Color::srgb(0.5, 0.5, 0.5)
    }

    /// Entity kind for targeting/UI. Default `Neutral`.
    fn entity_kind() -> EntityKind {
        EntityKind::Neutral
    }

    /// Initial movement, combat, and vital stats.
    fn stats() -> StatsBundleData {
        StatsBundleData {
            movement: crate::stats::components::MovementStats { speed: 0.15 },
            combat: crate::stats::components::CombatStats {
                attack_power: 10.0,
                armor: 0.0,
            },
            vital: crate::stats::components::VitalStats {
                current_health: 100.0,
                max_health: 100.0,
                max_mana: 100.0,
                mana_regeneration: 5.0,
            },
        }
    }

    /// Default lightyear replication target. Override only if a different target
    /// is needed (e.g. `NetworkTarget::AllExceptSingle(peer)`).
    fn replication_target() -> NetworkTarget {
        NetworkTarget::All
    }
}
