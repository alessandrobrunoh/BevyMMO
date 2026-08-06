//! Ray of Light spell implementation.
//!
//! Spell server-authoritative a forma di beam: parte dal caster, usa la
//! direzione in cui il caster sta guardando e infligge danno a **tutte** le
//! entità viventi dentro il cilindro retto davanti al caster, entro il range
//! massimo. Il client invia solo l'intenzione di cast; il server risolve target
//! e danno e replica solo l'effetto visivo.

use bevy::prelude::{Entity, Vec3};

use crate::plugins::spells::{Spell, SpellCastContext, SpellConfig, SpellId, TargetingMode};

/// Ray of Light: beam lineare che attraversa e danneggia ogni entità sulla linea.
pub struct RayOfLightSpell;

impl RayOfLightSpell {
    pub const ID: &'static str = "ray_of_light";
    pub const DISPLAY_NAME: &'static str = "Ray of Light";
    pub const COOLDOWN_SECONDS: f32 = 1.5;
    pub const CAST_TIME_SECONDS: f32 = 0.3;
    pub const CAST_RANGE: f32 = 8.0;
    /// Mezza larghezza del beam: determina quanto deve essere "largo" il ray.
    /// Poco inferiore al player per evitare hit spam laterali.
    pub const BEAM_HALF_WIDTH: f32 = 0.3;
    pub const DAMAGE_MULTIPLIER: f32 = 1.4;

    fn normalized_flat_direction(direction: Vec3) -> Vec3 {
        let flat = Vec3::new(direction.x, 0.0, direction.z);
        if flat.length_squared() > 0.001 {
            flat.normalize_or_zero()
        } else {
            Vec3::Z
        }
    }

    /// Restituisce ogni entità che si trova dentro il cilindro retto dal beam,
    /// insieme alla distanza lungo la direzione di tiro (utile per test e debug).
    ///
    /// A differenza di un projectile spell, non si ferma al primo target: il ray
    /// è pensato per attraversare e colpire tutto ciò che incontra.
    pub fn targets_along_line(
        caster: Entity,
        caster_position: Vec3,
        look_direction: Vec3,
        potential_targets: &[(Entity, Vec3)],
        max_range: f32,
        beam_half_width: f32,
    ) -> Vec<(Entity, f32)> {
        let direction = Self::normalized_flat_direction(look_direction);
        let mut hits: Vec<(Entity, f32)> = Vec::new();

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
            if lateral_distance > beam_half_width {
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
            Self::BEAM_HALF_WIDTH,
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
            RayOfLightSpell::BEAM_HALF_WIDTH,
        );
        hits.sort_by_key(|(_, distance)| distance.to_bits());

        let hit_entities: Vec<Entity> = hits.iter().map(|(entity, _)| *entity).collect();
        assert_eq!(hit_entities, vec![near, far]);
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
            RayOfLightSpell::BEAM_HALF_WIDTH,
        );

        assert!(hits.is_empty());
    }
}
