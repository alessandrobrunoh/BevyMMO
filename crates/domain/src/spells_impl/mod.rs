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

use std::sync::Arc;

use crate::spells::SpellRegistry;

/// Registers every spell definition available to the current game build.
///
/// This lives in `shared` so the client spellbook/HUD and the authoritative
/// server use the same registry. The first three are the default player
/// hotbar; the remaining entries are available for spellbook assignment and
/// boss/content systems.
/// Builds the registry containing every entry this build ships.
///
/// Returns the registry rather than filling a Bevy `Resource`: the
/// SpacetimeDB module has no `Startup` schedule and no ECS to put one in.
/// `bevymmo_shared` wraps this in a system for the client.
pub fn default_spells() -> SpellRegistry {
    #[allow(unused_mut)]
    let mut registry = SpellRegistry::default();
    registry.register(Arc::new(attack::AttackSpell));
    registry.register(Arc::new(fireball::FireballSpell));
    registry.register(Arc::new(healing_circle::HealingCircleSpell));
    registry.register(Arc::new(meteorite::MeteoriteSpell));
    registry.register(Arc::new(ray_of_light::RayOfLightSpell));
    registry.register(Arc::new(stun_field::StunFieldSpell));
    registry.register(Arc::new(swift::SwiftSpell));

    registry.register(Arc::new(dragon_enemy::cataclysm::CataclysmSpell));
    registry.register(Arc::new(dragon_enemy::dragon_claw::DragonClawSpell));
    registry.register(Arc::new(dragon_enemy::molten_eruption::MoltenEruptionSpell));
    registry.register(Arc::new(dragon_enemy::searing_breath::SearingBreathSpell));
    registry.register(Arc::new(dragon_enemy::tail_sweep::TailSweepSpell));
    registry.register(Arc::new(dragon_enemy::wing_buffet::WingBuffetSpell));
    registry
}
