//! Client visuals for Eidolon casts.
//!
//! Deliberately generic: the server sends only `spell_id` + start/end, and
//! everything else — forma, raggio, quanto dura il preavviso — viene riletto
//! dalla `BaseAbility` con quell'id, la stessa definizione che il server ha
//! usato per calcolare l'effetto. Aggiungere un gesto non richiede quindi di
//! scrivere un visual: la geometria ne sceglie già uno.
//!
//! Sono forme grezze (dischi, sfere, un sasso) pensate per i test: servono a
//! vedere DOVE e QUANDO un gesto colpisce, non a essere belle. Il colore
//! distingue i gesti fra loro finché non esistono VFX veri per Essenza.
//!
//! Nota: i raggi mostrati sono quelli base del gesto — i Modificatori incisi
//! (Espandere/Concentrare) cambiano l'area reale lato server ma non ancora
//! quella disegnata qui.

use bevy::color::Color;
use bevy::prelude::*;

use bevymmo_gameplay::abilities::{AbilityGeometry, ArcBaseAbility};
use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::effects::SpellVisual;

const BURST_SECONDS: f32 = 0.35;
const RING_IMPACT_SECONDS: f32 = 0.3;
const MUZZLE_SECONDS: f32 = 0.18;
const ROCK_FALL_SECONDS: f32 = 0.6;
const ROCK_START_HEIGHT: f32 = 9.0;
const CAST_HEIGHT: f32 = 1.1;

/// Sfera che si espande e svanisce sul punto d'impatto.
#[derive(Component)]
pub struct EidolonBurst {
    elapsed_seconds: f32,
    duration_seconds: f32,
    start_scale: f32,
    end_scale: f32,
}

/// Disco a terra: pulsa per tutta la finestra di preavviso, poi si allarga di
/// scatto nell'istante in cui il colpo arriva davvero.
#[derive(Component)]
pub struct EidolonGroundRing {
    elapsed_seconds: f32,
    warning_seconds: f32,
}

/// Il sasso del Meteorite: invisibile finché non è ora di cadere.
#[derive(Component)]
pub struct EidolonFallingRock {
    elapsed_seconds: f32,
    impact_at_seconds: f32,
    center: Vec3,
}

/// Colore del gesto. Tabellina volutamente esplicita: sono cinque gesti e
/// vedere a colpo d'occhio quale sta esplodendo vale più di una formula.
fn ability_color(id: &str) -> Color {
    match id {
        "arcane_orb" => Color::srgb(0.65, 0.45, 1.0),
        "arcane_seal" => Color::srgb(0.35, 0.6, 1.0),
        "binding_seal" => Color::srgb(1.0, 0.85, 0.25),
        "arcane_gale" => Color::srgb(0.4, 0.95, 0.9),
        "meteor_strike" => Color::srgb(1.0, 0.35, 0.15),
        _ => Color::srgb(1.0, 0.85, 0.5),
    }
}

fn glow(color: Color, strength: f32) -> LinearRgba {
    let rgba = color.to_linear();
    LinearRgba::rgb(
        rgba.red * strength,
        rgba.green * strength,
        rgba.blue * strength,
    )
}

/// Visual di un gesto Eidolon, scelto dalla sua geometria.
pub fn spawn_for_ability(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
    ability: &ArcBaseAbility,
) {
    let color = ability_color(effect.spell_id.as_str());
    let delay = ability.impact_delay();

    match ability.geometry() {
        AbilityGeometry::Projectile { .. } => {
            // La palla è un'entità replicata che si vede da sé: qui basta il
            // lampo alla mano, che dà il feedback immediato del tasto.
            spawn_burst(
                commands,
                meshes,
                materials,
                effect.start + Vec3::Y * CAST_HEIGHT,
                color,
                MUZZLE_SECONDS,
                0.15,
                0.7,
            );
        }
        AbilityGeometry::Cone { .. } | AbilityGeometry::Circle { .. } => {
            let spec =
                crate::spells::ability_vfx::AbilityVfxSpec::from_ability(effect, ability.as_ref());
            crate::spells::ability_vfx::spawn_matching_footprint(
                commands, meshes, materials, &spec, color,
            );
            let radius = spec.radius.max(0.5);
            if delay <= 0.0 {
                spawn_burst(
                    commands,
                    meshes,
                    materials,
                    effect.start + Vec3::Y * 0.4,
                    color,
                    BURST_SECONDS,
                    0.2,
                    radius * 0.8,
                );
            }
            if delay >= ROCK_FALL_SECONDS {
                // Preavviso abbastanza lungo da far vedere qualcosa cadere.
                spawn_falling_rock(commands, meshes, materials, effect.start, color, delay);
            }
        }
        AbilityGeometry::SelfBuff { .. } => {
            spawn_burst(
                commands,
                meshes,
                materials,
                effect.start + Vec3::Y * CAST_HEIGHT,
                color,
                BURST_SECONDS,
                0.3,
                1.6,
            );
        }
    }
}

/// Ripiego per un `spell_id` che non corrisponde a nessuna `BaseAbility`.
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let color = ability_color(effect.spell_id.as_str());
    spawn_burst(
        commands,
        meshes,
        materials,
        effect.start + Vec3::Y * 0.5,
        color,
        BURST_SECONDS,
        0.1,
        1.5,
    );
}

fn spawn_burst(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    color: Color,
    duration_seconds: f32,
    start_scale: f32,
    end_scale: f32,
) {
    let mesh = meshes.add(Sphere::new(1.0));
    let material = materials.add(StandardMaterial {
        base_color: color.with_alpha(0.7),
        emissive: glow(color, 3.0),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(center).with_scale(Vec3::splat(start_scale)),
        SpellVisual,
        EidolonBurst {
            elapsed_seconds: 0.0,
            duration_seconds,
            start_scale,
            end_scale,
        },
    ));
}

#[allow(dead_code)]
fn spawn_ground_ring(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    radius: f32,
    color: Color,
    warning_seconds: f32,
) {
    let mesh = meshes.add(Cylinder::new(radius, 0.05));
    let material = materials.add(StandardMaterial {
        base_color: color.with_alpha(0.4),
        emissive: glow(color, 1.5),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        // `center.y` is the impact's real elevation (mountain, valley, or
        // sea level) — flattening it to a fixed height drew every ground
        // effect at world Y=0 regardless of where the caster was standing.
        Transform::from_translation(center + Vec3::Y * 0.05),
        SpellVisual,
        EidolonGroundRing {
            elapsed_seconds: 0.0,
            warning_seconds,
        },
    ));
}

fn spawn_falling_rock(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    color: Color,
    impact_at_seconds: f32,
) {
    let mesh = meshes.add(Sphere::new(0.8));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.12, 0.08),
        emissive: glow(color, 2.0),
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(center + Vec3::Y * ROCK_START_HEIGHT).with_scale(Vec3::ZERO),
        SpellVisual,
        EidolonFallingRock {
            elapsed_seconds: 0.0,
            impact_at_seconds,
            // Same reason as `spawn_ground_ring`: keep the real elevation, or
            // the rock lands at world Y=0 instead of the target's height.
            center,
        },
    ));
}

pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: ParamSet<(
        Query<(Entity, &mut Transform, &mut EidolonBurst)>,
        Query<(Entity, &mut Transform, &mut EidolonGroundRing)>,
        Query<(Entity, &mut Transform, &mut EidolonFallingRock)>,
    )>,
) {
    let delta = time.delta_secs();
    animate_bursts(delta, &mut commands, &mut visuals.p0());
    animate_ground_rings(delta, &mut commands, &mut visuals.p1());
    animate_falling_rocks(delta, &mut commands, &mut visuals.p2());
}

fn animate_bursts(
    delta: f32,
    commands: &mut Commands,
    bursts: &mut Query<(Entity, &mut Transform, &mut EidolonBurst)>,
) {
    for (entity, mut transform, mut burst) in bursts.iter_mut() {
        burst.elapsed_seconds += delta;
        if burst.elapsed_seconds >= burst.duration_seconds {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = (burst.elapsed_seconds / burst.duration_seconds).clamp(0.0, 1.0);
        transform.scale = Vec3::splat(burst.start_scale.lerp(burst.end_scale, progress));
    }
}

fn animate_ground_rings(
    delta: f32,
    commands: &mut Commands,
    rings: &mut Query<(Entity, &mut Transform, &mut EidolonGroundRing)>,
) {
    for (entity, mut transform, mut ring) in rings.iter_mut() {
        ring.elapsed_seconds += delta;
        let total = ring.warning_seconds + RING_IMPACT_SECONDS;
        if ring.elapsed_seconds >= total {
            commands.entity(entity).despawn();
            continue;
        }

        if ring.elapsed_seconds < ring.warning_seconds {
            // Pulsa mentre si aspetta: comunica "qui sta per succedere".
            let pulse = 1.0 + (ring.elapsed_seconds * 8.0).sin() * 0.05;
            transform.scale = Vec3::new(pulse, 1.0, pulse);
            continue;
        }

        let progress =
            ((ring.elapsed_seconds - ring.warning_seconds) / RING_IMPACT_SECONDS).clamp(0.0, 1.0);
        let burst = 1.0 + progress * 0.5;
        transform.scale = Vec3::new(burst, 1.0, burst);
    }
}

fn animate_falling_rocks(
    delta: f32,
    commands: &mut Commands,
    rocks: &mut Query<(Entity, &mut Transform, &mut EidolonFallingRock)>,
) {
    for (entity, mut transform, mut rock) in rocks.iter_mut() {
        rock.elapsed_seconds += delta;
        if rock.elapsed_seconds >= rock.impact_at_seconds {
            commands.entity(entity).despawn();
            continue;
        }

        let fall_start = rock.impact_at_seconds - ROCK_FALL_SECONDS;
        if rock.elapsed_seconds < fall_start {
            transform.scale = Vec3::ZERO;
            continue;
        }

        let progress = ((rock.elapsed_seconds - fall_start) / ROCK_FALL_SECONDS).clamp(0.0, 1.0);
        transform.translation = rock.center + Vec3::Y * ROCK_START_HEIGHT.lerp(0.7, progress);
        transform.scale = Vec3::splat(1.0 + progress * 0.25);
    }
}
