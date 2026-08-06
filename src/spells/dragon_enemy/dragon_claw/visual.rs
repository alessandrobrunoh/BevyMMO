//! Client visual for `dragon_claw`.
//!
//! Instant slash arc anchored at the hit target (`SpellVisualEffect.start`).
//! A thin yellow torus sliver sweeps briefly while fading; three small sparks
//! fan outward and shrink. No telegraph (the cast is instant), so the visual
//! only plays the impact flash.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::SpellVisualEffect;
use crate::plugins::spells::SpellVisual;
use crate::spells::dragon_enemy::EMBER_YELLOW;

/// Total lifetime of the slash arc in seconds.
const SLASH_SECONDS: f32 = 0.35;
/// Spark lifetime in seconds.
const SPARK_SECONDS: f32 = 0.30;
const SPARK_COUNT: usize = 3;
const SPARK_SPEED: f32 = 1.2;

#[derive(Component)]
pub struct ClawSlashVisual {
    elapsed_seconds: f32,
}

#[derive(Component)]
pub struct ClawSparkVisual {
    elapsed_seconds: f32,
    velocity: Vec3,
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn ease_in_cubic(t: f32) -> f32 {
    t.powi(3)
}

/// Spawns the slash arc and sparks at `effect.start`.
///
/// The slash is a flattened torus sliver that sweeps and fades; sparks fan
/// outward radially. Damage is server-authoritative and already resolved by the
/// time the replicated `SpellVisualEffect` arrives.
///
/// # Example
/// ```rust,ignore
/// visual::spawn(&mut commands, &mut meshes, &mut materials, &effect);
/// ```
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let anchor = effect.start + Vec3::Y * 1.0;

    let slash_mesh = meshes.add(Torus {
        major_radius: 1.2,
        minor_radius: 0.05,
    });
    let slash_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.85, 0.25, 0.85),
        emissive: EMBER_YELLOW,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(slash_mesh),
        MeshMaterial3d(slash_material),
        Transform::from_translation(anchor).with_scale(Vec3::ZERO),
        SpellVisual,
        ClawSlashVisual {
            elapsed_seconds: 0.0,
        },
    ));

    let spark_mesh = meshes.add(Sphere::new(0.08));
    let spark_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.25),
        emissive: EMBER_YELLOW,
        unlit: true,
        ..default()
    });

    for index in 0..SPARK_COUNT {
        // Spread sparks evenly on the XZ plane around the anchor.
        let angle = (index as f32 / SPARK_COUNT as f32) * std::f32::consts::TAU;
        let velocity = Vec3::new(angle.cos(), 0.4, angle.sin()) * SPARK_SPEED;
        commands.spawn((
            Mesh3d(spark_mesh.clone()),
            MeshMaterial3d(spark_material.clone()),
            Transform::from_translation(anchor),
            SpellVisual,
            ClawSparkVisual {
                elapsed_seconds: 0.0,
                velocity,
            },
        ));
    }
}

/// Animates the slash sweep/fade and the sparks.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, visual::animate);
/// ```
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: ParamSet<(
        Query<(Entity, &mut Transform, &mut ClawSlashVisual)>,
        Query<(Entity, &mut Transform, &mut ClawSparkVisual)>,
    )>,
) {
    let delta = time.delta_secs();
    animate_slashes(delta, &mut commands, &mut visuals.p0());
    animate_sparks(delta, &mut commands, &mut visuals.p1());
}

fn animate_slashes(
    delta: f32,
    commands: &mut Commands,
    slashes: &mut Query<(Entity, &mut Transform, &mut ClawSlashVisual)>,
) {
    for (entity, mut transform, mut visual) in slashes.iter_mut() {
        visual.elapsed_seconds += delta;
        let t = visual.elapsed_seconds;

        if t >= SLASH_SECONDS {
            commands.entity(entity).despawn();
            continue;
        }

        // Grow phase: 0 -> 0.10s, scale 0 -> 1 (ease out).
        if t < 0.10 {
            let growth = ease_out_cubic(t / 0.10);
            // Flatten the torus into a sliver and scale with growth.
            transform.scale = Vec3::new(growth, growth, growth * 0.25);
        } else {
            // Hold full scale; alpha fade is approximated by shrinking minor
            // (Z) so the sliver thins out toward the end.
            let fade_progress = (t - 0.10) / (SLASH_SECONDS - 0.10);
            let fade = 1.0 - ease_in_cubic(fade_progress);
            transform.scale = Vec3::new(1.0, 1.0, fade * 0.25);
        }

        // Continuous sweep around Y: +90° over the full lifetime.
        let sweep = (t / SLASH_SECONDS) * std::f32::consts::FRAC_PI_2;
        transform.rotation = Quat::from_rotation_y(sweep);
    }
}

fn animate_sparks(
    delta: f32,
    commands: &mut Commands,
    sparks: &mut Query<(Entity, &mut Transform, &mut ClawSparkVisual)>,
) {
    for (entity, mut transform, mut spark) in sparks.iter_mut() {
        spark.elapsed_seconds += delta;
        let t = spark.elapsed_seconds;

        if t >= SPARK_SECONDS {
            commands.entity(entity).despawn();
            continue;
        }

        // Drift outward with a light gravity drop baked into the velocity Y.
        transform.translation += spark.velocity * delta;
        // Fake gravity by pulling velocity Y down each tick.
        spark.velocity.y -= 4.0 * delta;

        // Shrink to zero.
        let shrink = 1.0 - ease_in_cubic(t / SPARK_SECONDS);
        transform.scale = Vec3::splat(shrink.max(0.0));
    }
}
