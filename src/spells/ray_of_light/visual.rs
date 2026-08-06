//! Client-side visual per la spell "Ray of Light".
//!
//! Il server risolve start/end del beam (sempre lungo la direzione di sguardo
//! del caster, fino al range massimo). Il client costruisce un raggio giallo
//! emissivo che collega i due punti, fatto svanire rapidamente. Il danno è già
//! stato applicato lato server: questa entità è puramente decorativa.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::SpellVisualEffect;
use crate::plugins::spells::SpellVisual;

/// Durata del flash del beam. Corta: il ray è un lampo, non un effetto persistente.
pub const LIFETIME_SECONDS: f32 = 0.18;

/// Spessore verticale del beam. Molto sottile per dare l'idea di una lama di luce.
const BEAM_HEIGHT: f32 = 0.15;
/// Larghezza del beam. Poco inferiore al player (come richiesto dal design).
const BEAM_WIDTH: f32 = 0.6;
/// Solleva il beam dal pavimento per allinearlo al centro-massa del caster.
const SPAWN_HEIGHT_OFFSET: f32 = 0.8;
/// Trasparenza iniziale del beam. L'animazione la fade-out fino a 0.
const INITIAL_ALPHA: f32 = 0.95;

#[derive(Component)]
pub struct RayOfLightVisual {
    elapsed_seconds: f32,
    initial_alpha: f32,
}

/// Costruisce un beam locale dal segmento risolto dal server.
///
/// Il visual non ha autorità sul danno: replica solo il segmento comunicato dal
/// server così che il client veda il ray fermarsi esattamente al range massimo.
///
/// # Example
/// ```rust,ignore
/// spawn(&mut commands, &mut meshes, &mut materials, &effect);
/// ```
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let start = effect.start + Vec3::Y * SPAWN_HEIGHT_OFFSET;
    let end = effect.end + Vec3::Y * SPAWN_HEIGHT_OFFSET;
    let displacement = end - start;
    let length = displacement.length();
    let unit_length = length.max(0.001);

    let mesh = meshes.add(Cuboid::new(BEAM_WIDTH, BEAM_HEIGHT, unit_length));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.95, 0.2, INITIAL_ALPHA),
        emissive: LinearRgba::rgb(1.0, 0.9, 0.15),
        unlit: true,
        ..default()
    });

    // Orientamento: il Cuboid è lungo Z, quindi allineiamo +Z al displacement.
    let forward = displacement / unit_length;
    let orientation = Quat::from_rotation_arc(Vec3::Z, forward);
    // Centriamo il beam a metà tra start ed end.
    let midpoint = start + displacement * 0.5;

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(midpoint).with_rotation(orientation),
        SpellVisual,
        RayOfLightVisual {
            elapsed_seconds: 0.0,
            initial_alpha: INITIAL_ALPHA,
        },
    ));
}

/// Fa svanire il beam fino a despawn. Sistema main-world only.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, animate);
/// ```
#[allow(clippy::type_complexity)]
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        &mut RayOfLightVisual,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let delta = time.delta().as_secs_f32();
    for (entity, material_handle, mut visual) in visuals.iter_mut() {
        visual.elapsed_seconds += delta;
        let progress = (visual.elapsed_seconds / LIFETIME_SECONDS).clamp(0.0, 1.0);

        let Some(mut material) = materials.get_mut(material_handle) else {
            // Fallback di sicurezza: despawn se il materiale è stato deallocato.
            if progress >= 1.0 {
                commands.entity(entity).despawn();
            }
            continue;
        };

        let alpha = visual.initial_alpha * (1.0 - progress);
        material.base_color.set_alpha(alpha.max(0.0));

        if progress >= 1.0 {
            commands.entity(entity).despawn();
        }
    }
}
