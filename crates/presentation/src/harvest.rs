//! Client-only fill visual for harvestable nodes: scale and tint follow pieces.

use bevy::prelude::*;
use bevymmo_gameplay::gathering::Harvestable;
use bevymmo_gameplay::placeables::{KindId, PlaceableRegistry};

use crate::renderer::RenderedEntity;

/// Scale at `current_pieces == 0` relative to the authored transform.
pub const DEPLETED_SCALE: f32 = 0.85;
/// RGB multiplier applied after desaturation when the node is empty.
const DEPLETED_BRIGHTNESS: f32 = 0.4;

/// `current / max`, clamped to `[0, 1]`. Missing or zero max is empty.
pub fn harvest_fill_ratio(current_pieces: u32, max_pieces: u32) -> f32 {
    if max_pieces == 0 {
        0.0
    } else {
        (current_pieces as f32 / max_pieces as f32).clamp(0.0, 1.0)
    }
}

/// Uniform scale factor: [`DEPLETED_SCALE`] when empty, `1` when full.
pub fn harvest_visual_scale(ratio: f32) -> f32 {
    let ratio = ratio.clamp(0.0, 1.0);
    DEPLETED_SCALE + (1.0 - DEPLETED_SCALE) * ratio
}

/// Darkens and desaturates `original` as `ratio` drops toward empty.
pub fn harvest_tinted_color(original: Color, ratio: f32) -> Color {
    let ratio = ratio.clamp(0.0, 1.0);
    let [r, g, b, a] = original.to_srgba().to_f32_array();
    let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let mul = DEPLETED_BRIGHTNESS + (1.0 - DEPLETED_BRIGHTNESS) * ratio;
    Color::srgba(
        (luma + (r - luma) * ratio) * mul,
        (luma + (g - luma) * ratio) * mul,
        (luma + (b - luma) * ratio) * mul,
        a,
    )
}

/// Authored scale captured before fill is applied, so regen can restore it.
#[derive(Component)]
pub(crate) struct HarvestableFillVisual {
    authored_scale: Vec3,
}

/// Per-mesh private material so depleting one node does not tint shared glTF
/// (or fallback-cube) materials used by every other instance.
#[derive(Component)]
pub(crate) struct HarvestableMeshTint {
    shared: Handle<StandardMaterial>,
    cloned: Handle<StandardMaterial>,
    original_color: Color,
}

pub(crate) fn update_harvestable_fill(
    mut commands: Commands,
    placeables: Option<Res<PlaceableRegistry>>,
    mut nodes: Query<
        (
            Entity,
            &Harvestable,
            &mut Transform,
            Option<&HarvestableFillVisual>,
        ),
        With<RenderedEntity>,
    >,
    children: Query<&Children>,
    mesh_materials: Query<&MeshMaterial3d<StandardMaterial>>,
    tints: Query<&HarvestableMeshTint>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(placeables) = placeables else {
        return;
    };
    for (entity, harvestable, mut transform, visual) in &mut nodes {
        let Some(max_pieces) = max_pieces_for(&harvestable.kind_id, &placeables) else {
            continue;
        };
        let ratio = harvest_fill_ratio(harvestable.current_pieces, max_pieces);
        let authored = visual
            .map(|visual| visual.authored_scale)
            .unwrap_or(transform.scale);
        transform.scale = authored * harvest_visual_scale(ratio);
        if visual.is_none() {
            commands.entity(entity).insert(HarvestableFillVisual {
                authored_scale: authored,
            });
        }

        for_each_descendant_inclusive(entity, &children, &mut |mesh_entity| {
            apply_mesh_tint(
                mesh_entity,
                ratio,
                &mut commands,
                &mesh_materials,
                &tints,
                &mut materials,
            );
        });
    }
}

fn max_pieces_for(kind_id: &str, registry: &PlaceableRegistry) -> Option<u32> {
    registry
        .resources
        .get(&KindId::new(kind_id.to_owned()))
        .map(|definition| definition.resource_config().max_pieces)
        .filter(|max| *max > 0)
}

fn for_each_descendant_inclusive(
    entity: Entity,
    children: &Query<&Children>,
    visit: &mut impl FnMut(Entity),
) {
    visit(entity);
    if let Ok(kids) = children.get(entity) {
        for child in kids.iter() {
            for_each_descendant_inclusive(child, children, visit);
        }
    }
}

fn apply_mesh_tint(
    entity: Entity,
    ratio: f32,
    commands: &mut Commands,
    mesh_materials: &Query<&MeshMaterial3d<StandardMaterial>>,
    tints: &Query<&HarvestableMeshTint>,
    materials: &mut Assets<StandardMaterial>,
) {
    let Ok(handle) = mesh_materials.get(entity) else {
        return;
    };
    let full = (ratio - 1.0).abs() < f32::EPSILON;

    if let Ok(tint) = tints.get(entity) {
        if full {
            if handle.0 != tint.shared {
                commands
                    .entity(entity)
                    .insert(MeshMaterial3d(tint.shared.clone()));
            }
            return;
        }
        if let Some(mut material) = materials.get_mut(&tint.cloned) {
            material.base_color = harvest_tinted_color(tint.original_color, ratio);
        }
        if handle.0 != tint.cloned {
            commands
                .entity(entity)
                .insert(MeshMaterial3d(tint.cloned.clone()));
        }
        return;
    }

    if full {
        return;
    }
    let Some(source) = materials.get(&handle.0).cloned() else {
        return;
    };
    let original_color = source.base_color;
    let mut tinted = source;
    tinted.base_color = harvest_tinted_color(original_color, ratio);
    let cloned = materials.add(tinted);
    commands.entity(entity).insert((
        MeshMaterial3d(cloned.clone()),
        HarvestableMeshTint {
            shared: handle.0.clone(),
            cloned,
            original_color,
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_content::placeable_definitions;

    fn colors_close(a: Color, b: Color) -> bool {
        let a = a.to_srgba().to_f32_array();
        let b = b.to_srgba().to_f32_array();
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5)
    }

    #[test]
    fn fill_ratio_is_current_over_max() {
        assert!((harvest_fill_ratio(0, 10) - 0.0).abs() < f32::EPSILON);
        assert!((harvest_fill_ratio(5, 10) - 0.5).abs() < f32::EPSILON);
        assert!((harvest_fill_ratio(10, 10) - 1.0).abs() < f32::EPSILON);
        assert!((harvest_fill_ratio(3, 0) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn empty_node_is_smaller_full_node_is_authored_scale() {
        assert!((harvest_visual_scale(0.0) - DEPLETED_SCALE).abs() < 1e-5);
        assert!((harvest_visual_scale(1.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn full_tint_preserves_color_empty_is_darker_and_desaturated() {
        let green = Color::srgb(0.2, 0.8, 0.2);
        assert!(colors_close(harvest_tinted_color(green, 1.0), green));

        let empty = harvest_tinted_color(green, 0.0);
        let [r, g, b, _] = empty.to_srgba().to_f32_array();
        assert!(g < 0.5, "empty tint should be darker, got {g}");
        assert!(
            (r - g).abs() < 0.05 && (g - b).abs() < 0.05,
            "empty tint should be desaturated, got {r},{g},{b}"
        );
    }

    #[test]
    fn depleted_oak_shrinks_then_restores_at_full() {
        let mut app = App::new();
        let mut registry = PlaceableRegistry::default();
        placeable_definitions::register_all(&mut registry);
        app.insert_resource(registry);
        app.init_resource::<Assets<StandardMaterial>>();
        app.add_systems(Update, update_harvestable_fill);

        let entity = app
            .world_mut()
            .spawn((
                Harvestable {
                    placement_id: "oak_1".into(),
                    kind_id: "resource_oak_tree".into(),
                    current_pieces: 0,
                },
                Transform::default(),
                RenderedEntity,
            ))
            .id();

        app.update();
        let scale = app
            .world()
            .entity(entity)
            .get::<Transform>()
            .expect("transform")
            .scale;
        assert!(
            (scale.x - DEPLETED_SCALE).abs() < 1e-4,
            "depleted scale {scale}"
        );

        app.world_mut()
            .entity_mut(entity)
            .get_mut::<Harvestable>()
            .expect("harvestable")
            .current_pieces = 50;
        app.update();
        let scale = app
            .world()
            .entity(entity)
            .get::<Transform>()
            .expect("transform")
            .scale;
        assert!(
            (scale.x - 1.0).abs() < 1e-4,
            "full node must restore authored scale, got {scale}"
        );
    }
}
