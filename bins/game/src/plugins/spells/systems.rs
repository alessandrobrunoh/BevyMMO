//! Spell systems facade.
//!
//! The server-authoritative spell runtime now lives in `bevymmo_server`.
//! This root module keeps only the built-in spell registration function so the
//! existing plugin can keep composing server runtime + client presentation while
//! the crate-split migration is in progress.

use bevy::prelude::*;
use std::sync::Arc;

use bevymmo_shared::spells::{Spell, SpellRegistry};

pub use bevymmo_server::spells::systems::{
    advance_cast_progress, handle_cast_release, process_cast_requests, replicate_cast_progress,
    tick_spell_cooldowns,
};

/// Register all built-in spells at startup.
pub fn register_builtin_spells(mut registry: ResMut<SpellRegistry>) {
    bevy::log::info!("Registering built-in spells...");

    let attack_spell: Arc<dyn Spell> = Arc::new(crate::spells::attack::AttackSpell);
    registry.register(attack_spell);

    let ray_of_light_spell: Arc<dyn Spell> = Arc::new(crate::spells::ray_of_light::RayOfLightSpell);
    registry.register(ray_of_light_spell);

    let fireball_spell: Arc<dyn Spell> = Arc::new(crate::spells::fireball::FireballSpell);
    registry.register(fireball_spell);

    let healing_circle_spell: Arc<dyn Spell> =
        Arc::new(crate::spells::healing_circle::definition::HealingCircleSpell);
    registry.register(healing_circle_spell);

    let meteorite_spell: Arc<dyn Spell> = Arc::new(crate::spells::meteorite::MeteoriteSpell);
    registry.register(meteorite_spell);

    let stun_field_spell: Arc<dyn Spell> = Arc::new(crate::spells::stun_field::StunFieldSpell);
    registry.register(stun_field_spell);

    let swift_spell: Arc<dyn Spell> = Arc::new(crate::spells::swift::SwiftSpell);
    registry.register(swift_spell);

    let dragon_claw_spell: Arc<dyn Spell> =
        Arc::new(crate::spells::dragon_enemy::dragon_claw::DragonClawSpell);
    registry.register(dragon_claw_spell);

    let tail_sweep_spell: Arc<dyn Spell> =
        Arc::new(crate::spells::dragon_enemy::tail_sweep::TailSweepSpell);
    registry.register(tail_sweep_spell);

    let searing_breath_spell: Arc<dyn Spell> =
        Arc::new(crate::spells::dragon_enemy::searing_breath::SearingBreathSpell);
    registry.register(searing_breath_spell);

    let cinder_storm_spell: Arc<dyn Spell> =
        Arc::new(crate::spells::dragon_enemy::cinder_storm::CinderStormSpell);
    registry.register(cinder_storm_spell);

    let wing_buffet_spell: Arc<dyn Spell> =
        Arc::new(crate::spells::dragon_enemy::wing_buffet::WingBuffetSpell);
    registry.register(wing_buffet_spell);

    let molten_eruption_spell: Arc<dyn Spell> =
        Arc::new(crate::spells::dragon_enemy::molten_eruption::MoltenEruptionSpell);
    registry.register(molten_eruption_spell);

    let cataclysm_spell: Arc<dyn Spell> =
        Arc::new(crate::spells::dragon_enemy::cataclysm::CataclysmSpell);
    registry.register(cataclysm_spell);

    bevy::log::info!("Registered {} built-in spells", registry.len());
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_spell_cooldown_flow() {}
}
