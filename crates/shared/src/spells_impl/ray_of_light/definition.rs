//! Ray of Light spell implementation.
//!
//! Beam-shaped server-authoritative spell: originates from the caster, uses the
//! direction the caster is facing, and inflicts damage on **all** living
//! entities inside the right cylinder in front of the caster, up to the maximum
//! range. The client sends only the cast intent; the server resolves targets
//! and damage and replicates only the visual effect.

use bevy::prelude::{Entity, Vec3};

use crate::spells::{Spell, SpellCastContext, SpellConfig, SpellId, TargetingMode};

/// Ray of Light: linear beam that pierces and damages every entity in its path.
pub struct RayOfLightSpell;

impl RayOfLightSpell {
    pub const ID: &'static str = "ray_of_light";
    pub const DISPLAY_NAME: &'static str = "Ray of Light";
    pub const COOLDOWN_SECONDS: f32 = 1.5;
    pub const CAST_TIME_SECONDS: f32 = 0.3;
    pub const CAST_RANGE: f32 = 20.0;
    /// Horizontal hitbox radius around the beam center line.
    ///
    /// This mirrors the visible beam thickness closely enough for gameplay: a
    /// target whose body overlaps the ray should take damage even when its
    /// entity origin is not perfectly centered on the line.
    pub const BEAM_HITBOX_RADIUS: f32 = 0.6;
    pub const DAMAGE_MULTIPLIER: f32 = 1.4;

    fn normalized_flat_direction(direction: Vec3) -> Vec3 {
        let flat = Vec3::new(direction.x, 0.0, direction.z);
        if flat.length_squared() > 0.001 {
            flat.normalize_or_zero()
        } else {
            Vec3::Z
        }
    }

    /// Returns every entity located inside the beam's right cylinder,
    /// along with the distance along the shot direction (useful for testing and debugging).
    ///
    /// Unlike a projectile spell, it does not stop at the first target: the ray
    /// is designed to pierce and hit everything it encounters.
    pub fn targets_along_line(
        caster: Entity,
        caster_position: Vec3,
        look_direction: Vec3,
        potential_targets: &[(Entity, Vec3)],
        max_range: f32,
        beam_hitbox_radius: f32,
    ) -> Vec<(Entity, f32)> {
        let direction = Self::normalized_flat_direction(look_direction);
        let caster_flat_position = Vec3::new(caster_position.x, 0.0, caster_position.z);
        let mut hits: Vec<(Entity, f32)> = Vec::new();

        for (target, position) in potential_targets.iter().copied() {
            if target == caster {
                continue;
            }

            let target_flat_position = Vec3::new(position.x, 0.0, position.z);
            let to_target = target_flat_position - caster_flat_position;
            let forward_distance = to_target.dot(direction);
            if forward_distance <= 0.0 || forward_distance > max_range {
                continue;
            }

            let closest_point_on_line = caster_flat_position + direction * forward_distance;
            let lateral_distance = target_flat_position.distance(closest_point_on_line);
            if lateral_distance > beam_hitbox_radius {
                continue;
            }

            hits.push((target, forward_distance));
        }

        hits
    }
}

impl Spell for RayOfLightSpell {
    fn id(&self) -> SpellId {
        SpellId::new(Self::ID)
    }

    fn display_name(&self) -> &'static str {
        Self::DISPLAY_NAME
    }

    fn config(&self) -> SpellConfig {
        SpellConfig::ranged_single_target(
            Self::COOLDOWN_SECONDS,
            Self::CAST_RANGE,
            TargetingMode::DirectionalLine,
        )
        .with_cast_time(Self::CAST_TIME_SECONDS)
    }

    fn cast(&self, ctx: &mut SpellCastContext) {
        let direction = Self::normalized_flat_direction(ctx.caster_look_direction);
        let end = ctx.caster_position + direction * Self::CAST_RANGE;
        let damage = ctx.caster_combat.attack_power * Self::DAMAGE_MULTIPLIER;

        let hits = Self::targets_along_line(
            ctx.caster,
            ctx.caster_position,
            direction,
            ctx.potential_targets,
            Self::CAST_RANGE,
            Self::BEAM_HITBOX_RADIUS,
        );

        for (target, _) in hits {
            ctx.emit_damage(target, damage);
        }

        ctx.emit_visual(Self::ID, ctx.caster_position, end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_along_line_hits_every_entity_in_the_beam() {
        let caster = Entity::from_bits(1);
        let near = Entity::from_bits(2);
        let far = Entity::from_bits(3);
        let targets = [
            (near, Vec3::new(0.0, 0.0, 3.0)),
            (far, Vec3::new(0.0, 0.0, 6.0)),
        ];

        let mut hits = RayOfLightSpell::targets_along_line(
            caster,
            Vec3::ZERO,
            Vec3::Z,
            &targets,
            RayOfLightSpell::CAST_RANGE,
            RayOfLightSpell::BEAM_HITBOX_RADIUS,
        );
        hits.sort_by_key(|(_, distance)| distance.to_bits());

        let hit_entities: Vec<Entity> = hits.iter().map(|(entity, _)| *entity).collect();
        assert_eq!(hit_entities, vec![near, far]);
    }

    #[test]
    fn targets_along_line_uses_horizontal_beam_hitbox() {
        let caster = Entity::from_bits(1);
        let elevated = Entity::from_bits(2);
        let offset_inside_hitbox = Entity::from_bits(3);
        let targets = [
            (elevated, Vec3::new(0.0, 2.0, 8.0)),
            (offset_inside_hitbox, Vec3::new(0.55, 0.0, 8.0)),
        ];

        let hits = RayOfLightSpell::targets_along_line(
            caster,
            Vec3::ZERO,
            Vec3::Z,
            &targets,
            RayOfLightSpell::CAST_RANGE,
            RayOfLightSpell::BEAM_HITBOX_RADIUS,
        );
        let hit_entities: Vec<Entity> = hits.iter().map(|(entity, _)| *entity).collect();

        assert_eq!(hit_entities, vec![elevated, offset_inside_hitbox]);
    }

    #[test]
    fn targets_along_line_ignores_targets_behind_or_off_axis() {
        let caster = Entity::from_bits(1);
        let behind = Entity::from_bits(2);
        let side = Entity::from_bits(3);
        let targets = [
            (behind, Vec3::new(0.0, 0.0, -2.0)),
            (side, Vec3::new(2.5, 0.0, 3.0)),
        ];

        let hits = RayOfLightSpell::targets_along_line(
            caster,
            Vec3::ZERO,
            Vec3::Z,
            &targets,
            RayOfLightSpell::CAST_RANGE,
            RayOfLightSpell::BEAM_HITBOX_RADIUS,
        );

        assert!(hits.is_empty());
    }
}
