//! Geometric VFX registry and dispatcher for alpha abilities.
//!
//! Each of the 18 weapon-family abilities gets a dedicated spawn function
//! that produces a **distinct geometric manifestation** using Bevy primitive
//! meshes/materials. The registry maps `AbilityId` → spawn fn; the dispatcher
//! in `mod.rs` consults it before falling back to the legacy geometry-based
//! selector in `eidolon_effects`.

use bevy::color::Color;
use bevy::prelude::*;

use bevymmo_network::network::protocol::SpellVisualEffect;

use crate::spells::effects::SpellVisual;

// ---------------------------------------------------------------------------
// Individual ability modules
// ---------------------------------------------------------------------------
pub mod lifecycle;

pub mod arcane_bolt;
pub mod arcane_wave;
pub mod great_manifestation;
pub mod power_shot;
pub mod volley;
pub mod piercing_barrage;
pub mod cleave;
pub mod lunge;
pub mod blade_storm;
pub mod crushing_blow;
pub mod ground_slam;
pub mod cataclysm;
pub mod orb;
pub mod field;
pub mod domain;
pub mod strike;
pub mod rush;
pub mod impact;

// ---------------------------------------------------------------------------
// Registry types
// ---------------------------------------------------------------------------

/// Signature every ability-VFX spawn function must satisfy.
pub type AbilityVfxFn = fn(
    &mut Commands,
    &mut Assets<Mesh>,
    &mut Assets<StandardMaterial>,
    &SpellVisualEffect,
);

/// Runtime registry mapping ability ID → VFX spawn function.
#[derive(Resource, Default, Debug)]
pub struct AbilityVfxRegistry {
    map: std::collections::HashMap<&'static str, AbilityVfxFn>,
}

impl AbilityVfxRegistry {
    /// Register a spawn function for the given ability ID.
    pub fn register(&mut self, id: &'static str, fn_: AbilityVfxFn) {
        self.map.insert(id, fn_);
    }

    /// Look up the spawn function for an ability ID.
    pub fn get(&self, id: &str) -> Option<AbilityVfxFn> {
        self.map.get(id).copied()
    }

    /// Number of registered abilities (for testing / diagnostics).
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Population – called once during plugin setup
// ---------------------------------------------------------------------------

/// Fill the registry with all 18 alpha-ability entries.
pub fn populate_registry(registry: &mut AbilityVfxRegistry) {
    // Staff family
    registry.register("arcane_bolt", arcane_bolt::spawn);
    registry.register("arcane_wave", arcane_wave::spawn);
    registry.register("great_manifestation", great_manifestation::spawn);

    // Bow family
    registry.register("power_shot", power_shot::spawn);
    registry.register("volley", volley::spawn);
    registry.register("piercing_barrage", piercing_barrage::spawn);

    // Sword family
    registry.register("cleave", cleave::spawn);
    registry.register("lunge", lunge::spawn);
    registry.register("blade_storm", blade_storm::spawn);

    // Hammer family
    registry.register("crushing_blow", crushing_blow::spawn);
    registry.register("ground_slam", ground_slam::spawn);
    registry.register("cataclysm", cataclysm::spawn);

    // Focus family
    registry.register("orb", orb::spawn);
    registry.register("field", field::spawn);
    registry.register("domain", domain::spawn);

    // Gauntlets family
    registry.register("strike", strike::spawn);
    registry.register("rush", rush::spawn);
    registry.register("impact", impact::spawn);
}

// ---------------------------------------------------------------------------
// Animation system – ticks all lifecycle components each frame
// ---------------------------------------------------------------------------

/// Animate every ability-VFX entity that carries a lifecycle component.
pub fn animate_lifecycle(
    time: Res<Time>,
    mut commands: Commands,
    mut queries: ParamSet<(
        Query<(Entity, &mut Transform, &mut lifecycle::VfxExpandFade)>,
        Query<(Entity, &mut Transform, &mut lifecycle::VfxPulseRing)>,
        Query<(Entity, &mut lifecycle::VfxLifetime)>,
        Query<(Entity, &mut Transform, &mut lifecycle::VfxFall)>,
        Query<(Entity, &mut Transform, &mut lifecycle::VfxSpinExpand)>,
        Query<(Entity, &mut Transform, &mut lifecycle::VfxOscillate)>,
    )>,
) {
    let delta = time.delta_secs();

    for (entity, mut transform, mut comp) in queries.p0().iter_mut() {
        if comp.tick(delta, &mut transform) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut transform, mut comp) in queries.p1().iter_mut() {
        if comp.tick(delta, &mut transform) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut comp) in queries.p2().iter_mut() {
        if comp.tick(delta) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut transform, mut comp) in queries.p3().iter_mut() {
        if comp.tick(delta, &mut transform) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut transform, mut comp) in queries.p4().iter_mut() {
        if comp.tick(delta, &mut transform) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut transform, mut comp) in queries.p5().iter_mut() {
        if comp.tick(delta, &mut transform) {
            commands.entity(entity).despawn();
        }
    }
}

// ---------------------------------------------------------------------------
// Common helpers – reused across ability modules
// ---------------------------------------------------------------------------

/// Emissive glow from a base colour.
pub fn vfx_glow(color: Color, strength: f32) -> LinearRgba {
    let rgba = color.to_linear();
    LinearRgba::rgb(rgba.red * strength, rgba.green * strength, rgba.blue * strength)
}

/// Standard emissive-blend material used by most VFX meshes.
pub fn vfx_material(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
    alpha: f32,
    emissive_strength: f32,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color.with_alpha(alpha),
        emissive: vfx_glow(color, emissive_strength),
        alpha_mode: AlphaMode::Blend,
        ..default()
    })
}

/// Spawn a sphere mesh entity with [`SpellVisual`] marker + user-supplied lifecycle component.
///
/// This is the workhorse helper for "burst / expand / fade" style effects.
pub fn spawn_sphere<T: Component>(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    radius: f32,
    color: Color,
    alpha: f32,
    emissive: f32,
    scale: Vec3,
    lifecycle: T,
) {
    let mesh = meshes.add(Sphere::new(radius));
    let mat = vfx_material(materials, color, alpha, emissive);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(center).with_scale(scale),
        SpellVisual,
        lifecycle,
    ));
}

/// Spawn a horizontal cylinder (disc / ring) at ground level.
pub fn spawn_disc<T: Component>(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    radius: f32,
    height: f32,
    color: Color,
    alpha: f32,
    emissive: f32,
    scale: Vec3,
    lifecycle: T,
) {
    let mesh = meshes.add(Cylinder::new(radius, height));
    let mat = vfx_material(materials, color, alpha, emissive);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(center + Vec3::Y * 0.02).with_scale(scale),
        SpellVisual,
        lifecycle,
    ));
}

/// Spawn a box (cuboid) mesh – useful for blade / shockwave shapes.
pub fn spawn_box<T: Component>(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    size: Vec3,
    color: Color,
    alpha: f32,
    emissive: f32,
    lifecycle: T,
) {
    let mesh = meshes.add(Cuboid::from_size(size));
    let mat = vfx_material(materials, color, alpha, emissive);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(center),
        SpellVisual,
        lifecycle,
    ));
}

/// Spawn a capsule mesh – good for lunges, rushes, elongated strikes.
pub fn spawn_capsule<T: Component>(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    radius: f32,
    length: f32,
    color: Color,
    alpha: f32,
    emissive: f32,
    lifecycle: T,
) {
    let mesh = meshes.add(Capsule3d::new(radius, length));
    let mat = vfx_material(materials, color, alpha, emissive);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(center),
        SpellVisual,
        lifecycle,
    ));
}

/// Spawn a torus (ring) mesh – for orbital / domain effects.
pub fn spawn_torus<T: Component>(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    ring_radius: f32,
    tube_radius: f32,
    color: Color,
    alpha: f32,
    emissive: f32,
    lifecycle: T,
) {
    let mesh = meshes.add(Torus::new(ring_radius, tube_radius));
    let mat = vfx_material(materials, color, alpha, emissive);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(center),
        SpellVisual,
        lifecycle,
    ));
}

/// Spawn a cone mesh – for directional AoE visualisation (ground slam wave, etc.).
pub fn spawn_cone<T: Component>(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    radius: f32,
    height: f32,
    color: Color,
    alpha: f32,
    emissive: f32,
    lifecycle: T,
) {
    let mesh = meshes.add(Cone { radius, height });
    let mat = vfx_material(materials, color, alpha, emissive);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(center),
        SpellVisual,
        lifecycle,
    ));
}

/// Spawn a tetrahedron (sharp, angular) – for piercing / kinetic effects.
pub fn spawn_tetra<T: Component>(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    size: f32,
    color: Color,
    alpha: f32,
    emissive: f32,
    lifecycle: T,
) {
    // Bevy doesn't have Tetrahedron primitive; use a small sharp box as proxy
    // or compose from custom vertices. For now we use a scaled box to represent
    // a sharp kinetic shape.
    let mesh = meshes.add(Cuboid::from_size(Vec3::splat(size)));
    let mat = vfx_material(materials, color, alpha, emissive);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(center),
        SpellVisual,
        lifecycle,
    ));
}

/// Colour palette per weapon family (used as base; each ability tweaks it).
mod palette {
    use bevy::color::Color;

    pub const STAFF: Color = Color::srgb(0.65, 0.45, 1.0);       // violet
    pub const BOW: Color = Color::srgb(0.3, 0.9, 0.5);          // emerald
    pub const SWORD: Color = Color::srgb(1.0, 0.85, 0.2);       // gold
    pub const HAMMER: Color = Color::srgb(1.0, 0.4, 0.15);      // fire orange
    pub const FOCUS: Color = Color::srgb(0.35, 0.6, 1.0);       // azure
    pub const GAUNTLETS: Color = Color::srgb(1.0, 0.45, 0.6);   // rose
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_all_18_abilities() {
        let mut reg = AbilityVfxRegistry::default();
        populate_registry(&mut reg);
        assert_eq!(reg.len(), 18);
    }

    #[test]
    fn registry_lookup_succeeds_for_each_ability() {
        let mut reg = AbilityVfxRegistry::default();
        populate_registry(&mut reg);

        let expected = [
            "arcane_bolt", "arcane_wave", "great_manifestation",
            "power_shot", "volley", "piercing_barrage",
            "cleave", "lunge", "blade_storm",
            "crushing_blow", "ground_slam", "cataclysm",
            "orb", "field", "domain",
            "strike", "rush", "impact",
        ];
        for id in expected {
            assert!(reg.get(id).is_some(), "{id} should be registered");
        }
    }

    #[test]
    fn registry_returns_none_for_unknown() {
        let reg = AbilityVfxRegistry::default();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn registry_all_fns_are_distinct_pointers() {
        let mut reg = AbilityVfxRegistry::default();
        populate_registry(&mut reg);

        // Collect all fn pointers and verify no duplicates (each ability has its own)
        let fns: Vec<_> = reg.map.values().collect();
        let unique: std::collections::HashSet<_> = fns.iter().copied().collect();
        assert_eq!(unique.len(), fns.len(), "each ability must have its own spawn fn");
    }
}
