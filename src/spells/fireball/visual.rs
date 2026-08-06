//! Visual client-side per la spell Fireball.
//!
//! Il dispatcher in `plugins::spells::effects` chiama `spawn` quando arriva un
//! `SpellVisualEffect` con l'id di Fireball. L'animazione è registrata nello
//! stesso punto.
//!
//! A differenza di un'interpolazione `start -> end`, la fireball è modellata
//! come proiettile a velocità costante: continua a viaggiare nella direzione
//! di lancio finché non scade il `LIFETIME_SECONDS`. Questo evita il despawn
//! immediato appena raggiunge la posizione di impatto e le permette di
//! attraversare la scena per minuti interi.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::SpellVisualEffect;
use crate::plugins::spells::SpellVisual;

/// Tempo totale di vita del proiettile visivo.
///
/// Tenuto volutamente lungo (20 minuti) per evitare che la fireball sparisca
/// poco dopo il cast: il danno viene applicato istantaneamente lato server,
/// ma la rappresentazione visiva deve persistere come un proiettile in volo.
pub const LIFETIME_SECONDS: f32 = 20.0 * 60.0;

/// Velocità di volo in unità mondo al secondo.
pub const SPEED: f32 = 8.0;

const SIZE: f32 = 0.28;
const SPAWN_HEIGHT_OFFSET: f32 = 0.8;

#[derive(Component)]
pub struct FireballVisual {
    velocity: Vec3,
    elapsed_seconds: f32,
}

/// Spawn della rappresentazione visiva di Fireball.
///
/// La direzione di volo è derivata dal vettore `start -> end` fornito dal
/// server; il proiettile poi continua dritto per `LIFETIME_SECONDS`.
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let mesh = meshes.add(Cuboid::new(SIZE, SIZE, SIZE));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.35, 0.05),
        emissive: LinearRgba::rgb(1.0, 0.25, 0.02),
        ..default()
    });

    let start = effect.start + Vec3::Y * SPAWN_HEIGHT_OFFSET;
    let direction = (effect.end - effect.start)
        .try_normalize()
        .unwrap_or(Vec3::Z);

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(start),
        SpellVisual,
        FireballVisual {
            velocity: direction * SPEED,
            elapsed_seconds: 0.0,
        },
    ));
}

/// Anima le entità `FireballVisual` muovendole in linea retta e le despawna
/// solo allo scadere del lifetime.
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: Query<(Entity, &mut Transform, &mut FireballVisual)>,
) {
    let delta = time.delta().as_secs_f32();
    for (entity, mut transform, mut visual) in visuals.iter_mut() {
        visual.elapsed_seconds += delta;
        transform.translation += visual.velocity * delta;

        if visual.elapsed_seconds >= LIFETIME_SECONDS {
            commands.entity(entity).despawn();
        }
    }
}
