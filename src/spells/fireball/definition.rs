//! Fireball spell implementation.
//!
//! Spell frontale server-authoritative: parte dal caster, usa la direzione in
//! cui il caster sta guardando e colpisce la prima entità viva davanti entro
//! range. Il client invia solo l'intenzione di cast, non decide il target.

use bevy::prelude::{Entity, Vec3};

use crate::plugins::spells::{Spell, SpellCastContext, SpellConfig, SpellId};

/// Fireball: projectile-like ranged AoE spell.
pub struct FireballSpell;

impl FireballSpell {
    pub const ID: &'static str = "fireball";
    pub const DISPLAY_NAME: &'static str = "Fireball";
    pub const COOLDOWN_SECONDS: f32 = 1.2;
    pub const CAST_RANGE: f32 = 8.0;
    pub const HIT_RADIUS: f32 = 1.0;
    pub const DAMAGE_MULTIPLIER: f32 = 1.4;

    fn normalized_flat_direction(direction: Vec3) -> Vec3 {
        let flat = Vec3::new(direction.x, 0.0, direction.z);
        if flat.length_squared() > 0.001 {
            flat.normalize_or_zero()
        } else {
            Vec3::Z
        }
    }

    /// Trova il primo target lungo una linea di tiro frontale.
    pub fn first_target_in_front(
        caster: Entity,
        caster_position: Vec3,
        look_direction: Vec3,
        potential_targets: &[(Entity, Vec3)],
        max_range: f32,
        hit_radius: f32,
    ) -> Option<(Entity, Vec3, f32)> {
        let direction = Self::normalized_flat_direction(look_direction);
        let mut closest_hit: Option<(Entity, Vec3, f32)> = None;

        for (target, position) in potential_targets.iter().copied() {
            if target == caster {
                continue;
            }

            let to_target = position - caster_position;
            let forward_distance = to_target.dot(direction);
            if forward_distance <= 0.0 || forward_distance > max_range {
                continue;
            }

            let closest_point_on_line = caster_position + direction * forward_distance;
            let lateral_distance = position.distance(closest_point_on_line);
            if lateral_distance > hit_radius {
                continue;
            }

            match closest_hit {
                None => closest_hit = Some((target, position, forward_distance)),
                Some((_, _, current_distance)) if forward_distance < current_distance => {
                    closest_hit = Some((target, position, forward_distance));
                }
                _ => {}
            }
        }

        closest_hit
    }
}

impl Spell for FireballSpell {
    fn id(&self) -> SpellId {
        SpellId::new(Self::ID)
    }

    fn display_name(&self) -> &'static str {
        Self::DISPLAY_NAME
    }

    fn config(&self) -> SpellConfig {
        SpellConfig::ranged_single_target(Self::COOLDOWN_SECONDS, Self::CAST_RANGE)
    }

    fn cast(&self, ctx: &mut SpellCastContext) {
        let direction = Self::normalized_flat_direction(ctx.caster_look_direction);
        let fallback_end = ctx.caster_position + direction * Self::CAST_RANGE;
        let damage = ctx.caster_combat.attack_power * Self::DAMAGE_MULTIPLIER;

        let end = if let Some((target, hit_position, _)) = Self::first_target_in_front(
            ctx.caster,
            ctx.caster_position,
            direction,
            ctx.potential_targets,
            Self::CAST_RANGE,
            Self::HIT_RADIUS,
        ) {
            ctx.emit_damage(target, damage);
            hit_position
        } else {
            fallback_end
        };

        ctx.emit_visual(Self::ID, ctx.caster_position, end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_target_in_front_selects_closest_target_on_line() {
        let caster = Entity::from_bits(1);
        let near = Entity::from_bits(2);
        let far = Entity::from_bits(3);
        let targets = [
            (far, Vec3::new(0.0, 0.0, 6.0)),
            (near, Vec3::new(0.0, 0.0, 3.0)),
        ];

        let hit = FireballSpell::first_target_in_front(
            caster,
            Vec3::ZERO,
            Vec3::Z,
            &targets,
            FireballSpell::CAST_RANGE,
            FireballSpell::HIT_RADIUS,
        );

        assert_eq!(hit.map(|(entity, _, _)| entity), Some(near));
    }

    #[test]
    fn first_target_in_front_ignores_targets_behind_or_too_far_lateral() {
        let caster = Entity::from_bits(1);
        let behind = Entity::from_bits(2);
        let side = Entity::from_bits(3);
        let targets = [
            (behind, Vec3::new(0.0, 0.0, -2.0)),
            (side, Vec3::new(2.5, 0.0, 3.0)),
        ];

        let hit = FireballSpell::first_target_in_front(
            caster,
            Vec3::ZERO,
            Vec3::Z,
            &targets,
            FireballSpell::CAST_RANGE,
            FireballSpell::HIT_RADIUS,
        );

        assert!(hit.is_none());
    }
}
