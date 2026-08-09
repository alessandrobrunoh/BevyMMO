//! Procedural macro for defining game props with minimal boilerplate.
//!
//! # Example
//!
//! ```rust,ignore
//! use bevymmo_shared::placeables::props;
//!
//! // Tree: cylindrical trunk collider, blocks movement.
//! #[props(
//!     id = "tree_oak_large",
//!     name = "Oak Tree (Large)",
//!     icon = "🌳",
//!     asset = "models/new/tree_oak_large.glb",
//!     scale = (1.0, 1.0, 1.0),
//!     tint = (0.2, 0.5, 0.2),
//!     blocks_movement = true,
//!     collision = cylinder(radius = 0.4, height = 6.0)
//! )]
//! pub struct TreeOakLargeProp;
//!
//! // Decorative prop: no collision, never blocks.
//! #[props(
//!     id = "pebbles",
//!     name = "Pebbles",
//!     icon = "🪨",
//!     asset = "models/new/pebbles.glb",
//!     blocks_movement = false
//! )]
//! pub struct PebblesProp;
//! ```

use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::parse_macro_input;
use syn::DeriveInput;

/// Attribute macro to define a placeable prop with minimal boilerplate.
///
/// Automatically implements:
/// - `PlaceableDefinition`
/// - `PropPlaceable`
/// - `register()` function
#[proc_macro_attribute]
pub fn props(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    // Parse attribute arguments manually (syn 2.x compatible)
    let attr_str = attr.to_string();
    let attrs = parse_attrs(&attr_str);

    // Extract struct name
    let name = &input.ident;

    // Get values with defaults
    let id = attrs.id.unwrap_or_else(|| name.to_string().to_lowercase());
    let display_name = attrs.display_name.unwrap_or_else(|| id.clone());
    let icon = attrs.icon.unwrap_or_else(|| "❓".to_string());
    let asset = attrs.asset.unwrap_or_else(|| format!("models/{}.glb", id));

    // Destructure tuples for quote! (tuples don't implement ToTokens)
    let (scale_x, scale_y, scale_z) = attrs.scale.unwrap_or((1.0_f32, 1.0_f32, 1.0_f32));
    let (tint_r, tint_g, tint_b) = attrs.tint.unwrap_or((0.5_f32, 0.5_f32, 0.5_f32));
    let blocks_movement = attrs.blocks_movement;

    // Translate the parsed collision DSL into a `CollisionShape` constructor
    // token stream. We intentionally keep the macro self-contained and do not
    // depend on the `CollisionShape` path being imported at the call site:
    // we emit a fully-qualified path so prop authors only need `use ...props`.
    let collision_tokens = build_collision_tokens(attrs.collision);

    // Generate implementation
    let expanded = quote! {
        /// Auto-generated prop definition via #[props] macro.
        #input

        impl crate::placeables::PlaceableDefinition for #name {
            fn id(&self) -> crate::placeables::KindId {
                crate::placeables::KindId::new(#id)
            }

            fn display_name(&self) -> &'static str {
                #display_name
            }

            fn icon(&self) -> &'static str {
                #icon
            }

            fn asset_hint(&self) -> crate::placeables::AssetHint {
                crate::placeables::AssetHint::Scene(#asset)
            }

            fn defaults(&self) -> crate::placeables::PlaceableDefaults {
                crate::placeables::PlaceableDefaults {
                    transform: crate::world::TransformData {
                        translation: [0.0_f32, 0.0_f32, 0.0_f32],
                        rotation_deg: [0.0_f32, 0.0_f32, 0.0_f32],
                        scale: [#scale_x, #scale_y, #scale_z],
                    },
                    tint: Some([#tint_r, #tint_g, #tint_b]),
                    collision: #collision_tokens,
                    blocks_movement: #blocks_movement,
                }
            }
        }

        impl crate::placeables::PropPlaceable for #name {}

        /// Register this prop in the global registry.
        pub fn register(registry: &mut crate::placeables::PlaceableRegistry) {
            registry.register_prop(std::sync::Arc::new(#name))
        }
    };

    TokenStream::from(expanded)
}

/// Parsed `collision = ...` clause.
///
/// Kept as a small enum (instead of pre-rendering tokens) so that the macro
/// stays readable and so future shapes can be added in one place.
enum CollisionSpec {
    None,
    Cylinder {
        radius: f32,
        height: f32,
    },
    Box {
        half_x: f32,
        half_y: f32,
        half_z: f32,
    },
    Sphere {
        radius: f32,
    },
}

/// Emits the Rust expression that constructs the `CollisionShape` (or `None`
/// when no collision was specified).
///
/// The output is used verbatim as the `collision:` field initializer in
/// `PlaceableDefaults`, so it must evaluate to `Option<CollisionShape>`.
fn build_collision_tokens(spec: CollisionSpec) -> proc_macro2::TokenStream {
    use quote::quote;

    let path = quote!(crate::world::CollisionShape);
    match spec {
        CollisionSpec::None => quote!(None),
        CollisionSpec::Cylinder { radius, height } => {
            quote!(Some(#path::Cylinder { radius: #radius, height: #height }))
        }
        CollisionSpec::Box {
            half_x,
            half_y,
            half_z,
        } => {
            quote!(Some(#path::Box { half_extents: [#half_x, #half_y, #half_z] }))
        }
        CollisionSpec::Sphere { radius } => {
            quote!(Some(#path::Sphere { radius: #radius }))
        }
    }
}

/// Simple attribute parser for syn 2.x compatibility
struct PropsAttributes {
    id: Option<String>,
    display_name: Option<String>,
    icon: Option<String>,
    asset: Option<String>,
    scale: Option<(f32, f32, f32)>,
    tint: Option<(f32, f32, f32)>,
    blocks_movement: bool,
    collision: CollisionSpec,
}

fn parse_attrs(input: &str) -> PropsAttributes {
    let mut props = PropsAttributes {
        id: None,
        display_name: None,
        icon: None,
        asset: None,
        scale: None,
        tint: None,
        blocks_movement: false,
        collision: CollisionSpec::None,
    };

    // The naive "split by comma" used previously breaks the moment the
    // `collision = ...` clause itself contains commas (e.g.
    // `cylinder(radius = 0.4, height = 6.0)`). To stay compatible with the
    // rest of the parser we split on top-level commas only, ignoring commas
    // nested inside parentheses.
    let pairs: Vec<String> = split_top_level_commas(input);

    for pair in pairs.iter() {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }

        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        match key {
            "id" => props.id = Some(clean_string(value)),
            "name" => props.display_name = Some(clean_string(value)),
            "icon" => props.icon = Some(clean_string(value)),
            "asset" => props.asset = Some(clean_string(value)),
            "blocks_movement" => {
                props.blocks_movement = value == "true";
            }
            "scale" => {
                props.scale = parse_triple_f32(value);
            }
            "tint" => {
                props.tint = parse_triple_f32(value);
            }
            "collision" => {
                props.collision = parse_collision_spec(value);
            }
            _ => {}
        }
    }

    props
}

/// Splits the macro attribute string on commas that are *not* nested inside
/// parentheses. Required to support
/// `collision = cylinder(radius = 0.4, height = 6.0)` where the inner commas
/// belong to the collision DSL and must not split the key/value pair.
fn split_top_level_commas(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth: i32 = 0;
    let mut current = String::new();
    for ch in input.chars() {
        match ch {
            '(' | '[' | '{' => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut current));
            }
            c => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        out.push(current);
    }
    out
}

/// Parses the collision DSL value (e.g. `"cylinder(radius = 0.4, height = 6.0)"`)
/// into a [`CollisionSpec`]. Returns [`CollisionSpec::None`] for unknown
/// shapes so a typo degrades to "no collision" instead of failing the build.
fn parse_collision_spec(value: &str) -> CollisionSpec {
    let value = value.trim();
    let Some((kind, body)) = value.split_once('(') else {
        return CollisionSpec::None;
    };
    let kind = kind.trim();
    // Strip the trailing ')'.
    let body = body.trim_end();
    let body = body.strip_suffix(')').unwrap_or(body);
    let fields = parse_kv_fields(body);

    match kind {
        "cylinder" => {
            let radius = fields.get("radius").and_then(|v| v.parse::<f32>().ok());
            let height = fields.get("height").and_then(|v| v.parse::<f32>().ok());
            match (radius, height) {
                (Some(radius), Some(height)) => CollisionSpec::Cylinder { radius, height },
                _ => CollisionSpec::None,
            }
        }
        "box" => {
            // Two accepted spellings: explicit half-extents triple, or
            // (width, height, depth) which we halve automatically.
            if let Some(half) = fields.get("half_extents").and_then(|s| parse_triple_f32(s)) {
                return CollisionSpec::Box {
                    half_x: half.0,
                    half_y: half.1,
                    half_z: half.2,
                };
            }
            let dims = parse_triple_f32(body);
            dims.map(|(x, y, z)| CollisionSpec::Box {
                half_x: x * 0.5,
                half_y: y * 0.5,
                half_z: z * 0.5,
            })
            .unwrap_or(CollisionSpec::None)
        }
        "sphere" => fields
            .get("radius")
            .and_then(|v| v.parse::<f32>().ok())
            .map(|radius| CollisionSpec::Sphere { radius })
            .unwrap_or(CollisionSpec::None),
        _ => CollisionSpec::None,
    }
}

/// Parses a `key = value, key = value` body (without the surrounding parens)
/// into a small lookup table. Keys are case-sensitive and the last occurrence
/// of a duplicated key wins, matching the typical DSL behaviour.
fn parse_kv_fields(body: &str) -> HashMap<&str, &str> {
    body.split(',')
        .filter_map(|entry| {
            let entry = entry.trim();
            let (k, v) = entry.split_once('=')?;
            Some((k.trim(), v.trim()))
        })
        .collect()
}

fn clean_string(s: &str) -> String {
    s.trim_matches('"').to_string()
}

fn parse_triple_f32(s: &str) -> Option<(f32, f32, f32)> {
    // Parse "(1.0, 2.0, 3.0)"
    let inner = s.trim_matches('(').trim_matches(')');
    let parts: Vec<&str> = inner.split(',').collect();

    if parts.len() == 3 {
        let x: f32 = parts[0].trim().parse().ok()?;
        let y: f32 = parts[1].trim().parse().ok()?;
        let z: f32 = parts[2].trim().parse().ok()?;
        Some((x, y, z))
    } else {
        None
    }
}
