//! Procedural macro for defining game props with minimal boilerplate.
//!
//! # Example
//!
//! ```rust,ignore
//! use bevymmo_shared::placeables::props;
//!
//! #[props(
//!     id = "rock",
//!     name = "Rock",
//!     icon = "🪨",
//!     asset = "models/rock.glb",
//!     scale = (1.0, 1.0, 1.0),
//!     tint = (0.4, 0.38, 0.35),
//!     blocks_movement = true
//! )]
//! pub struct RockProp;
//! ```

use proc_macro::TokenStream;
use quote::quote;
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
                    collision: None,
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

/// Simple attribute parser for syn 2.x compatibility
struct PropsAttributes {
    id: Option<String>,
    display_name: Option<String>,
    icon: Option<String>,
    asset: Option<String>,
    scale: Option<(f32, f32, f32)>,
    tint: Option<(f32, f32, f32)>,
    blocks_movement: bool,
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
    };

    // Very simple parser - split by commas, then by "="
    // This handles basic cases like: id = "rock", name = "Rock"
    let pairs: Vec<&str> = input.split(',').collect();

    for pair in pairs {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }

        if let Some((key, value)) = pair.split_once('=') {
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
                _ => {}
            }
        }
    }

    props
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
