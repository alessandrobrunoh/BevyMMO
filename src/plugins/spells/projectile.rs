//! Sistema server-authoritative per i proiettili homing.
//!
//! Modulo parallelo a [`crate::plugins::spells::aoe`]: gestisce il ciclo di
//! vita di qualsiasi entità dotata del componente [`HomingProjectile`]. Il
//! payload dell'effetto (damage, hit_radius, speed) è già portato dal
//! componente stesso, quindi il sistema è del tutto agnostico rispetto alla
//! spell che ha spawnato il proiettile (oggi solo `Followball`, domani
//! potenzialmente altre spell homing).

use bevy::prelude::*;

use crate::network::protocol::Position;
use crate::stats::components::VitalStats;
use crate::stats::events::DamageEvent;

/// Component marker per un proiettile homing: insegue `target` a `speed`,
/// applica `damage` quando entra in `hit_radius`, poi despawna.
#[derive(Component, Debug)]
pub struct HomingProjectile {
    pub target: Entity,
    pub speed: f32,
    pub damage: f32,
    pub hit_radius: f32,
}

/// Sistema server-authoritative: muove i proiettili homing verso il target
/// e applica danno all'impatto.
///
/// Le due query sono rese disgiunte da `Without<HomingProjectile>`: i target
/// non possono essere proiettili stessi, evitando il conflitto B0001.
pub fn update_homing_projectiles(
    time: Res<Time>,
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Position, &HomingProjectile)>,
    targets: Query<(&Position, &VitalStats), Without<HomingProjectile>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    for (proj_entity, mut proj_pos, proj) in projectiles.iter_mut() {
        // Se il target non esiste più, non ha Position/VitalStats o è morto, despawn.
        let Ok((target_pos, target_vital)) = targets.get(proj.target) else {
            commands.entity(proj_entity).despawn();
            continue;
        };
        if target_vital.is_dead() {
            commands.entity(proj_entity).despawn();
            continue;
        }

        let direction = target_pos.0 - proj_pos.0;
        let distance = direction.length();

        // Hit: abbastanza vicino da colpire
        if distance <= proj.hit_radius {
            damage_events.write(DamageEvent {
                target: proj.target,
                source: None,
                amount: proj.damage,
            });
            commands.entity(proj_entity).despawn();
            continue;
        }

        // Muovi verso il target. `speed` è espresso in unità/secondo.
        let step = (proj.speed * time.delta_secs()).min(distance);
        proj_pos.0 += direction / distance * step;
    }
}
