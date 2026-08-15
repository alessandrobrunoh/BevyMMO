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
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::collections::HashMap;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{bracketed, parenthesized, parse_macro_input};
use syn::{DeriveInput, Ident, LitFloat, LitInt, LitStr, Path, Token};

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

// ============================================================================
// #[item(...)] — declares a concrete `bevymmo_shared::items::Item`.
// ============================================================================
//
// Same spirit as `#[props(...)]` above (a unit struct + a small DSL generates
// the whole trait impl), but parsed with `syn::parse::Parse` instead of the
// manual string splitter, because the DSL here nests (`effects = [...]`,
// `spells(q = [...], w = [...], e = ...)`) and hand-rolled comma-splitting
// does not compose well past one level of nesting.
//
// Optionally declares the Q/W/E spell kit an item grants while equipped
// (`bevymmo_shared::items::SpellKit`, see `crates/shared/src/items/spell_kit.rs`):
// if the `spells(...)` clause is present, the macro rejects at compile time
// any shape that isn't "at least one Q, at least one W, exactly one E" — the
// contract required by the hotbar (`crate::spells::components::SpellHotbar`)
// — instead of only catching it at startup.
//
// # Example
// ```ignore
// use bevymmo_props_macro::item;
//
// // A weapon that grants two Q options, one W option, one E spell.
// #[item(
//     id = "magic_staff",
//     name = "Flame Staff",
//     description = "Forgiato nel cuore di un vulcano dormiente.",
//     category = Weapon,
//     rarity = Rare,
//     slot = Weapon,
//     effects = [stat_bonus(field = AttackPower, op = Add, value = 25.0)],
//     spells(
//         q = [AttackSpell, FireballSpell],
//         w = [StunFieldSpell],
//         e = MeteoriteSpell,
//     ),
// )]
// pub struct MagicStaff;
//
// // A pure stat item — no `spells(...)` clause, `spell_kit()` stays `None`.
// #[item(
//     id = "iron_plate_armor_v2",
//     name = "Corazza di Ferro",
//     category = Armor,
//     rarity = Common,
//     slot = Armor,
//     effects = [stat_bonus(field = MaxHealth, op = Add, value = 500.0)],
// )]
// pub struct IronPlateArmorV2;
// ```
//
// Numeric values (`value = 25.0`, `amount = 10.0`) must be written as float
// literals with a decimal point, matching the rest of the codebase's
// convention for `f32` constants.

#[proc_macro_attribute]
pub fn item(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    // Only unit structs are supported: like `#[props(...)]`, all data comes
    // from macro arguments (stored in `static OnceLock`s in the generated
    // impl), not from struct fields.
    if let syn::Data::Struct(data) = &input.data {
        if !matches!(data.fields, syn::Fields::Unit) {
            return syn::Error::new_spanned(
                &input,
                "#[item(...)] can only be applied to a unit struct, e.g. `pub struct MagicStaff;` \
                 (all item data comes from the macro arguments, not from struct fields)",
            )
            .to_compile_error()
            .into();
        }
    } else {
        return syn::Error::new_spanned(&input, "#[item(...)] can only be applied to a struct")
            .to_compile_error()
            .into();
    }

    let def = parse_macro_input!(attr as ItemDef);
    TokenStream::from(def.build_tokens(&input))
}

/// One `key = value` pair inside `stat_bonus(...)` / `instant_heal(...)`.
/// The value is either a bare identifier (`field = MaxHealth`) or a float
/// literal (`value = 25.0`) — which one is expected depends on the key, so
/// [`KvPair::ident_value`] / [`KvPair::float_value`] check that at use time
/// and report a clear error if the wrong kind was written.
struct KvPair {
    key: Ident,
    value: KvValue,
}

enum KvValue {
    Ident(Ident),
    Float(LitFloat),
}

impl Parse for KvPair {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value = if input.peek(LitFloat) {
            KvValue::Float(input.parse()?)
        } else {
            KvValue::Ident(input.parse()?)
        };
        Ok(Self { key, value })
    }
}

impl KvPair {
    fn ident_value(&self) -> syn::Result<Ident> {
        match &self.value {
            KvValue::Ident(ident) => Ok(ident.clone()),
            KvValue::Float(lit) => Err(syn::Error::new_spanned(
                lit,
                format!("expected an identifier for `{}`, found a number", self.key),
            )),
        }
    }

    fn float_value(&self) -> syn::Result<LitFloat> {
        match &self.value {
            KvValue::Float(lit) => Ok(lit.clone()),
            KvValue::Ident(ident) => Err(syn::Error::new_spanned(
                ident,
                format!("expected a number for `{}`, found an identifier", self.key),
            )),
        }
    }
}

/// One entry of `effects = [...]`.
enum EffectDef {
    StatBonus { field: Ident, op: Ident, value: LitFloat },
    InstantHeal { amount: LitFloat },
}

impl Parse for EffectDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let kind: Ident = input.parse()?;
        let content;
        parenthesized!(content in input);
        let pairs: Punctuated<KvPair, Token![,]> = Punctuated::parse_terminated(&content)?;

        match kind.to_string().as_str() {
            "stat_bonus" => {
                let mut field = None;
                let mut op = None;
                let mut value = None;
                for pair in &pairs {
                    match pair.key.to_string().as_str() {
                        "field" => field = Some(pair.ident_value()?),
                        "op" => op = Some(pair.ident_value()?),
                        "value" => value = Some(pair.float_value()?),
                        other => {
                            return Err(syn::Error::new_spanned(
                                &pair.key,
                                format!("unknown key `{other}` in stat_bonus(...) (expected field, op, value)"),
                            ))
                        }
                    }
                }
                Ok(EffectDef::StatBonus {
                    field: field
                        .ok_or_else(|| syn::Error::new_spanned(&kind, "stat_bonus(...) requires `field = ...`"))?,
                    op: op.ok_or_else(|| syn::Error::new_spanned(&kind, "stat_bonus(...) requires `op = ...`"))?,
                    value: value
                        .ok_or_else(|| syn::Error::new_spanned(&kind, "stat_bonus(...) requires `value = ...`"))?,
                })
            }
            "instant_heal" => {
                let mut amount = None;
                for pair in &pairs {
                    if pair.key == "amount" {
                        amount = Some(pair.float_value()?);
                    } else {
                        return Err(syn::Error::new_spanned(
                            &pair.key,
                            format!("unknown key `{}` in instant_heal(...) (expected amount)", pair.key),
                        ));
                    }
                }
                Ok(EffectDef::InstantHeal {
                    amount: amount
                        .ok_or_else(|| syn::Error::new_spanned(&kind, "instant_heal(...) requires `amount = ...`"))?,
                })
            }
            other => Err(syn::Error::new_spanned(
                &kind,
                format!("unknown effect `{other}` in effects = [...] (expected stat_bonus, instant_heal)"),
            )),
        }
    }
}

/// Parsed `spells(q = [...], w = [...], e = ...)` clause.
///
/// This is where the Q(1+) / W(1+) / E(1) contract is enforced: parsing
/// fails with a `syn::Error` (surfaced as a normal compile error at the
/// macro call site) unless `q` and `w` each have at least one entry and `e`
/// has exactly one.
struct SpellsDef {
    q: Vec<Path>,
    w: Vec<Path>,
    e: Path,
}

impl SpellsDef {
    fn parse_from(content: ParseStream) -> syn::Result<Self> {
        let mut q: Option<Vec<Path>> = None;
        let mut w: Option<Vec<Path>> = None;
        let mut e: Option<Path> = None;

        while !content.is_empty() {
            let key: Ident = content.parse()?;
            content.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "q" => {
                    let inner;
                    bracketed!(inner in content);
                    let list: Punctuated<Path, Token![,]> = Punctuated::parse_terminated(&inner)?;
                    q = Some(list.into_iter().collect());
                }
                "w" => {
                    let inner;
                    bracketed!(inner in content);
                    let list: Punctuated<Path, Token![,]> = Punctuated::parse_terminated(&inner)?;
                    w = Some(list.into_iter().collect());
                }
                "e" => e = Some(content.parse::<Path>()?),
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!("unknown key `{other}` in spells(...) (expected q, w, e)"),
                    ))
                }
            }
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        let q = q.unwrap_or_default();
        let w = w.unwrap_or_default();

        if q.is_empty() {
            return Err(syn::Error::new(
                content.span(),
                "spells(...) requires at least one spell in `q = [...]` — every item that grants \
                 spells must offer at least one Q option",
            ));
        }
        if w.is_empty() {
            return Err(syn::Error::new(
                content.span(),
                "spells(...) requires at least one spell in `w = [...]`",
            ));
        }
        let e = e.ok_or_else(|| {
            syn::Error::new(
                content.span(),
                "spells(...) requires exactly one spell in `e = ...` (found none)",
            )
        })?;

        Ok(Self { q, w, e })
    }
}

/// Parsed `abilities(primary = [X, Y], secondary = [Z], ultimate = W)`
/// clause — the "Eidolon" model (gesti offerti dall'arma, plasmati dalla
/// frase incisa dal giocatore), alternativo a `spells(...)` (menu di spell
/// pronte). Un item usa l'uno O l'altro, mai entrambi. Stesso vincolo di
/// `SpellsDef`: Primary(1+)/Secondary(1+)/Ultimate(1) — il giocatore sceglie
/// UNA fra le opzioni di Primary e UNA fra quelle di Secondary a runtime
/// (vedi `AbilitySelection`); Ultimate non ha scelta perché ne offre solo una.
struct AbilitiesDef {
    primary: Vec<Path>,
    secondary: Vec<Path>,
    ultimate: Path,
}

impl AbilitiesDef {
    fn parse_from(content: ParseStream) -> syn::Result<Self> {
        let mut primary: Option<Vec<Path>> = None;
        let mut secondary: Option<Vec<Path>> = None;
        let mut ultimate = None;

        while !content.is_empty() {
            let key: Ident = content.parse()?;
            content.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "primary" => {
                    let inner;
                    bracketed!(inner in content);
                    let list: Punctuated<Path, Token![,]> = Punctuated::parse_terminated(&inner)?;
                    primary = Some(list.into_iter().collect());
                }
                "secondary" => {
                    let inner;
                    bracketed!(inner in content);
                    let list: Punctuated<Path, Token![,]> = Punctuated::parse_terminated(&inner)?;
                    secondary = Some(list.into_iter().collect());
                }
                "ultimate" => ultimate = Some(content.parse::<Path>()?),
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!("unknown key `{other}` in abilities(...) (expected primary, secondary, ultimate)"),
                    ))
                }
            }
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        let primary = primary.unwrap_or_default();
        let secondary = secondary.unwrap_or_default();

        if primary.is_empty() {
            return Err(syn::Error::new(
                content.span(),
                "abilities(...) requires at least one gesto in `primary = [...]`",
            ));
        }
        if secondary.is_empty() {
            return Err(syn::Error::new(
                content.span(),
                "abilities(...) requires at least one gesto in `secondary = [...]`",
            ));
        }
        let ultimate = ultimate.ok_or_else(|| {
            syn::Error::new(content.span(), "abilities(...) requires exactly one gesto in `ultimate = ...`")
        })?;

        Ok(Self { primary, secondary, ultimate })
    }
}

/// Parsed `rune_profile(capacity = ..., stability = ..., affinity = ...)`.
/// Required alongside `abilities(...)` — un'arma "Eidolon" senza profilo
/// runico non potrebbe mai essere incisa.
struct RuneProfileDef {
    capacity: LitInt,
    stability: LitFloat,
    affinity: Option<Ident>,
}

impl RuneProfileDef {
    fn parse_from(content: ParseStream) -> syn::Result<Self> {
        let mut capacity = None;
        let mut stability = None;
        let mut affinity = None;

        while !content.is_empty() {
            let key: Ident = content.parse()?;
            content.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "capacity" => capacity = Some(content.parse::<LitInt>()?),
                "stability" => stability = Some(content.parse::<LitFloat>()?),
                "affinity" => affinity = Some(content.parse::<Ident>()?),
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!("unknown key `{other}` in rune_profile(...) (expected capacity, stability, affinity)"),
                    ))
                }
            }
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(Self {
            capacity: capacity.ok_or_else(|| content.error("rune_profile(...) requires `capacity = ...`"))?,
            stability: stability.ok_or_else(|| content.error("rune_profile(...) requires `stability = ...`"))?,
            affinity,
        })
    }
}

/// Fully parsed `#[item(...)]` argument list.
struct ItemDef {
    id: LitStr,
    name: LitStr,
    description: Option<LitStr>,
    category: Ident,
    rarity: Ident,
    slot: Option<Ident>,
    effects: Vec<EffectDef>,
    spells: Option<SpellsDef>,
    abilities: Option<AbilitiesDef>,
    rune_profile: Option<RuneProfileDef>,
}

impl Parse for ItemDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut id = None;
        let mut name = None;
        let mut description = None;
        let mut category = None;
        let mut rarity = None;
        let mut slot = None;
        let mut effects = Vec::new();
        let mut spells = None;
        let mut abilities = None;
        let mut rune_profile = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let key_str = key.to_string();

            if key_str == "spells" {
                // `spells(...)` reads like a function call, no `=` before it.
                let content;
                parenthesized!(content in input);
                spells = Some(SpellsDef::parse_from(&content)?);
            } else if key_str == "abilities" {
                let content;
                parenthesized!(content in input);
                abilities = Some(AbilitiesDef::parse_from(&content)?);
            } else if key_str == "rune_profile" {
                let content;
                parenthesized!(content in input);
                rune_profile = Some(RuneProfileDef::parse_from(&content)?);
            } else {
                input.parse::<Token![=]>()?;
                match key_str.as_str() {
                    "id" => id = Some(input.parse::<LitStr>()?),
                    "name" => name = Some(input.parse::<LitStr>()?),
                    "description" => description = Some(input.parse::<LitStr>()?),
                    "category" => category = Some(input.parse::<Ident>()?),
                    "rarity" => rarity = Some(input.parse::<Ident>()?),
                    "slot" => slot = Some(input.parse::<Ident>()?),
                    "effects" => {
                        let content;
                        bracketed!(content in input);
                        let list: Punctuated<EffectDef, Token![,]> = Punctuated::parse_terminated(&content)?;
                        effects = list.into_iter().collect();
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            &key,
                            format!(
                                "unknown key `{other}` in #[item(...)] (expected id, name, description, \
                                 category, rarity, slot, effects, spells, abilities, rune_profile)"
                            ),
                        ))
                    }
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        if abilities.is_some() != rune_profile.is_some() {
            return Err(input.error(
                "#[item(...)] requires `abilities(...)` and `rune_profile(...)` together — an Eidolon \
                 weapon without a rune profile could never be inscribed, and a rune profile without \
                 abilities has nothing to inscribe onto",
            ));
        }

        Ok(Self {
            id: id.ok_or_else(|| input.error("#[item(...)] requires `id = \"...\"`"))?,
            name: name.ok_or_else(|| input.error("#[item(...)] requires `name = \"...\"`"))?,
            description,
            category: category.ok_or_else(|| {
                input.error(
                    "#[item(...)] requires `category = ...` (Weapon | Armor | Consumable | Material | Quest | Accessory)",
                )
            })?,
            rarity: rarity.ok_or_else(|| {
                input.error("#[item(...)] requires `rarity = ...` (Common | Uncommon | Rare | Epic | Legendary)")
            })?,
            slot,
            effects,
            spells,
            abilities,
            rune_profile,
        })
    }
}

impl ItemDef {
    fn build_tokens(&self, original: &DeriveInput) -> TokenStream2 {
        let name = &original.ident;
        let id_lit = &self.id;
        let display_name_lit = &self.name;
        let description_lit = self
            .description
            .clone()
            .unwrap_or_else(|| LitStr::new("", proc_macro2::Span::call_site()));
        let category = &self.category;
        let rarity = &self.rarity;
        let equippable_into = match &self.slot {
            Some(slot) => quote! { Some(crate::items::EquipSlot::#slot) },
            None => quote! { None },
        };

        let effect_tokens: Vec<TokenStream2> = self
            .effects
            .iter()
            .map(|effect| match effect {
                EffectDef::StatBonus { field, op, value } => quote! {
                    crate::items::ItemEffect::StatBonus {
                        field: crate::stats::events::StatField::#field,
                        op: crate::stats::events::ModifierOp::#op,
                        value: #value,
                    }
                },
                EffectDef::InstantHeal { amount } => quote! {
                    crate::items::ItemEffect::InstantHeal { amount: #amount }
                },
            })
            .collect();

        // Only override `spell_kit()` when a `spells(...)` clause was given;
        // otherwise the trait's `None` default (see
        // `crates/shared/src/items/definition.rs`) applies, exactly like a
        // hand-written item that never mentions spells.
        let spell_kit_method = self.spells.as_ref().map(|spells| {
            let q_paths = &spells.q;
            let w_paths = &spells.w;
            let e_path = &spells.e;
            quote! {
                fn spell_kit(&self) -> Option<&crate::items::SpellKit> {
                    static KIT: std::sync::OnceLock<crate::items::SpellKit> = std::sync::OnceLock::new();
                    Some(KIT.get_or_init(|| crate::items::SpellKit::new(
                        vec![#(crate::spells::SpellId::new(#q_paths::ID)),*],
                        vec![#(crate::spells::SpellId::new(#w_paths::ID)),*],
                        crate::spells::SpellId::new(#e_path::ID),
                    )))
                }
            }
        });

        // Only present for "Eidolon" weapons (`abilities(...)` given instead
        // of `spells(...)`): the fixed gesti + how much runic weight they
        // can sustain.
        let weapon_abilities_method = self.abilities.as_ref().map(|abilities| {
            let primary = &abilities.primary;
            let secondary = &abilities.secondary;
            let ultimate = &abilities.ultimate;
            quote! {
                fn weapon_abilities(&self) -> Option<&crate::abilities::WeaponAbilities> {
                    static ABILITIES: std::sync::OnceLock<crate::abilities::WeaponAbilities> = std::sync::OnceLock::new();
                    Some(ABILITIES.get_or_init(|| crate::abilities::WeaponAbilities::new(
                        vec![#(crate::abilities::AbilityId::new(#primary::ID)),*],
                        vec![#(crate::abilities::AbilityId::new(#secondary::ID)),*],
                        crate::abilities::AbilityId::new(#ultimate::ID),
                    )))
                }
            }
        });

        let rune_profile_method = self.rune_profile.as_ref().map(|profile| {
            let capacity = &profile.capacity;
            let stability = &profile.stability;
            let affinity_tokens = match &profile.affinity {
                Some(essence_ident) => {
                    let essence_id_lit = essence_ident.to_string();
                    quote! { Some(crate::abilities::EssenceId::new(#essence_id_lit)) }
                }
                None => quote! { None },
            };
            quote! {
                fn rune_profile(&self) -> Option<&crate::abilities::RuneProfile> {
                    static PROFILE: std::sync::OnceLock<crate::abilities::RuneProfile> = std::sync::OnceLock::new();
                    Some(PROFILE.get_or_init(|| crate::abilities::RuneProfile {
                        capacity: #capacity,
                        stability: #stability,
                        affinity: #affinity_tokens,
                    }))
                }
            }
        });

        quote! {
            #original

            impl #name {
                /// Stable id, generated by `#[item(...)]` so it stays in sync
                /// with `id()` below — mirrors the `pub const ID` convention
                /// used by every hand-written item (e.g. `IronSword::ID`).
                pub const ID: &'static str = #id_lit;
            }

            impl crate::items::Item for #name {
                fn id(&self) -> crate::items::ItemId {
                    crate::items::ItemId::new(Self::ID)
                }

                fn config(&self) -> &crate::items::ItemConfig {
                    static CONFIG: std::sync::OnceLock<crate::items::ItemConfig> = std::sync::OnceLock::new();
                    CONFIG.get_or_init(|| crate::items::ItemConfig {
                        display_name: std::borrow::Cow::Borrowed(#display_name_lit),
                        description: std::borrow::Cow::Borrowed(#description_lit),
                        category: crate::items::ItemCategory::#category,
                        rarity: crate::items::ItemRarity::#rarity,
                        equippable_into: #equippable_into,
                        weight: 0.0,
                    })
                }

                fn effects(&self) -> &[crate::items::ItemEffect] {
                    static EFFECTS: std::sync::OnceLock<Vec<crate::items::ItemEffect>> = std::sync::OnceLock::new();
                    EFFECTS.get_or_init(|| vec![#(#effect_tokens),*])
                }

                #spell_kit_method
                #weapon_abilities_method
                #rune_profile_method
            }

            impl #name {
                /// Registers this item in the global registry. Generated by `#[item(...)]`.
                pub fn register(registry: &mut crate::items::ItemRegistry) {
                    registry.register(std::sync::Arc::new(#name));
                }
            }
        }
    }
}

/// Rejects anything but a bare unit struct — every macro in this file only
/// generates trait impls from literals, never from struct fields.
fn require_unit_struct(input: &DeriveInput, macro_name: &str) -> Result<(), TokenStream> {
    let ok = matches!(&input.data, syn::Data::Struct(data) if matches!(data.fields, syn::Fields::Unit));
    if ok {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            input,
            format!(
                "#[{macro_name}(...)] can only be applied to a unit struct, e.g. `pub struct X;` \
                 (all data comes from the macro arguments, not from struct fields)"
            ),
        )
        .to_compile_error()
        .into())
    }
}

// ============================================================================
// #[base_ability(...)] — declares a `bevymmo_shared::abilities::BaseAbility`.
// ============================================================================
//
// Pure data, no behavior trait needed (unlike the three macros below):
// `BaseAbility::default_manifestation` already has a generic default derived
// from `geometry()`, so the macro can generate the ENTIRE `impl BaseAbility`.
//
// # Example
// ```ignore
// #[base_ability(
//     id = "staff_bolt", name = "Getto",
//     tags = [Ranged, Projectile, SingleTarget],
//     geometry = projectile(range = 20.0, speed = 26.0),
//     power = 260.0, cast_time = 0.35, cooldown = 4.0, energy_cost = 12.0,
//     animation = "staff_bolt_cast", impact_vfx = "bolt_impact_burst",
// )]
// pub struct StaffBolt;
// ```

enum GeometryDef {
    Cone { radius: LitFloat, angle_deg: LitFloat },
    /// `range` è la gittata entro cui si può piazzare il cerchio: assente
    /// (0.0) = il gesto esplode addosso a chi lo lancia.
    Circle { radius: LitFloat, range: Option<LitFloat> },
    Projectile { range: LitFloat, speed: LitFloat },
    SelfBuff { duration_seconds: LitFloat },
}

fn parse_geometry(kind: Ident, fields: Punctuated<KvPair, Token![,]>) -> syn::Result<GeometryDef> {
    let field = |name: &str| fields.iter().find(|p| p.key == name);
    match kind.to_string().as_str() {
        "cone" => Ok(GeometryDef::Cone {
            radius: field("radius")
                .ok_or_else(|| syn::Error::new_spanned(&kind, "cone(...) requires `radius = ...`"))?
                .float_value()?,
            angle_deg: field("angle_deg")
                .ok_or_else(|| syn::Error::new_spanned(&kind, "cone(...) requires `angle_deg = ...`"))?
                .float_value()?,
        }),
        "circle" => Ok(GeometryDef::Circle {
            radius: field("radius")
                .ok_or_else(|| syn::Error::new_spanned(&kind, "circle(...) requires `radius = ...`"))?
                .float_value()?,
            range: field("range").map(|pair| pair.float_value()).transpose()?,
        }),
        "projectile" => Ok(GeometryDef::Projectile {
            range: field("range")
                .ok_or_else(|| syn::Error::new_spanned(&kind, "projectile(...) requires `range = ...`"))?
                .float_value()?,
            speed: field("speed")
                .ok_or_else(|| syn::Error::new_spanned(&kind, "projectile(...) requires `speed = ...`"))?
                .float_value()?,
        }),
        "self_buff" => Ok(GeometryDef::SelfBuff {
            duration_seconds: field("duration_seconds")
                .ok_or_else(|| syn::Error::new_spanned(&kind, "self_buff(...) requires `duration_seconds = ...`"))?
                .float_value()?,
        }),
        other => Err(syn::Error::new_spanned(
            &kind,
            format!("unknown geometry `{other}` (expected cone, circle, projectile, self_buff)"),
        )),
    }
}

struct BaseAbilityDef {
    id: LitStr,
    name: LitStr,
    tags: Vec<Ident>,
    geometry: GeometryDef,
    power: LitFloat,
    cast_time: LitFloat,
    cooldown: LitFloat,
    energy_cost: LitFloat,
    animation: LitStr,
    impact_vfx: LitStr,
    /// Opzionali: assenti = impatto immediato e nessun controllo.
    impact_delay: Option<LitFloat>,
    stun_seconds: Option<LitFloat>,
}

impl Parse for BaseAbilityDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut id = None;
        let mut name = None;
        let mut tags = Vec::new();
        let mut geometry = None;
        let mut power = None;
        let mut cast_time = None;
        let mut cooldown = None;
        let mut energy_cost = None;
        let mut animation = None;
        let mut impact_vfx = None;
        let mut impact_delay = None;
        let mut stun_seconds = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "id" => id = Some(input.parse::<LitStr>()?),
                "name" => name = Some(input.parse::<LitStr>()?),
                "tags" => {
                    let content;
                    bracketed!(content in input);
                    let list: Punctuated<Ident, Token![,]> = Punctuated::parse_terminated(&content)?;
                    tags = list.into_iter().collect();
                }
                "geometry" => {
                    let kind: Ident = input.parse()?;
                    let content;
                    parenthesized!(content in input);
                    let fields: Punctuated<KvPair, Token![,]> = Punctuated::parse_terminated(&content)?;
                    geometry = Some(parse_geometry(kind, fields)?);
                }
                "power" => power = Some(input.parse::<LitFloat>()?),
                "cast_time" => cast_time = Some(input.parse::<LitFloat>()?),
                "cooldown" => cooldown = Some(input.parse::<LitFloat>()?),
                "energy_cost" => energy_cost = Some(input.parse::<LitFloat>()?),
                "animation" => animation = Some(input.parse::<LitStr>()?),
                "impact_vfx" => impact_vfx = Some(input.parse::<LitStr>()?),
                "impact_delay" => impact_delay = Some(input.parse::<LitFloat>()?),
                "stun_seconds" => stun_seconds = Some(input.parse::<LitFloat>()?),
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!(
                            "unknown key `{other}` in #[base_ability(...)] (expected id, name, tags, geometry, \
                             power, cast_time, cooldown, energy_cost, animation, impact_vfx, impact_delay, \
                             stun_seconds)"
                        ),
                    ))
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(Self {
            id: id.ok_or_else(|| input.error("#[base_ability(...)] requires `id = \"...\"`"))?,
            name: name.ok_or_else(|| input.error("#[base_ability(...)] requires `name = \"...\"`"))?,
            tags,
            geometry: geometry.ok_or_else(|| input.error("#[base_ability(...)] requires `geometry = ...`"))?,
            power: power.ok_or_else(|| input.error("#[base_ability(...)] requires `power = ...`"))?,
            cast_time: cast_time.ok_or_else(|| input.error("#[base_ability(...)] requires `cast_time = ...`"))?,
            cooldown: cooldown.ok_or_else(|| input.error("#[base_ability(...)] requires `cooldown = ...`"))?,
            energy_cost: energy_cost
                .ok_or_else(|| input.error("#[base_ability(...)] requires `energy_cost = ...`"))?,
            animation: animation.ok_or_else(|| input.error("#[base_ability(...)] requires `animation = \"...\"`"))?,
            impact_vfx: impact_vfx
                .ok_or_else(|| input.error("#[base_ability(...)] requires `impact_vfx = \"...\"`"))?,
            impact_delay,
            stun_seconds,
        })
    }
}

#[proc_macro_attribute]
pub fn base_ability(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    if let Err(err) = require_unit_struct(&input, "base_ability") {
        return err;
    }
    let def = parse_macro_input!(attr as BaseAbilityDef);
    let name = &input.ident;

    let id_lit = &def.id;
    let name_lit = &def.name;
    let tags = &def.tags;
    let power = &def.power;
    let cast_time = &def.cast_time;
    let cooldown = &def.cooldown;
    let energy_cost = &def.energy_cost;
    let animation = &def.animation;
    let impact_vfx = &def.impact_vfx;

    // `area`/`range` are derived from the geometry instead of being
    // separate clauses: the radius you already gave `cone`/`circle` IS the
    // area, so there is nothing left to restate.
    let (geometry_tokens, area, range) = match &def.geometry {
        GeometryDef::Cone { radius, angle_deg } => (
            quote! { crate::abilities::AbilityGeometry::Cone { radius: #radius, angle_deg: #angle_deg } },
            quote! { #radius },
            quote! { 0.0 },
        ),
        GeometryDef::Circle { radius, range } => {
            let range = match range {
                Some(range) => quote! { #range },
                None => quote! { 0.0 },
            };
            (
                quote! { crate::abilities::AbilityGeometry::Circle { radius: #radius } },
                quote! { #radius },
                range,
            )
        }
        GeometryDef::Projectile { range, speed } => (
            quote! { crate::abilities::AbilityGeometry::Projectile { range: #range, speed: #speed } },
            quote! { 0.0 },
            quote! { #range },
        ),
        GeometryDef::SelfBuff { duration_seconds } => (
            quote! { crate::abilities::AbilityGeometry::SelfBuff { duration_seconds: #duration_seconds } },
            quote! { 0.0 },
            quote! { 0.0 },
        ),
    };

    // Le due chiavi opzionali generano l'override solo se dichiarate: senza
    // di esse restano i default del trait (impatto immediato, nessuno stun).
    let impact_delay_method = match &def.impact_delay {
        Some(delay) => quote! {
            fn impact_delay(&self) -> f32 { #delay }
        },
        None => quote! {},
    };
    let stun_seconds_method = match &def.stun_seconds {
        Some(seconds) => quote! {
            fn stun_seconds(&self) -> f32 { #seconds }
        },
        None => quote! {},
    };

    let expanded = quote! {
        #input

        impl #name {
            pub const ID: &'static str = #id_lit;
        }

        impl crate::abilities::BaseAbility for #name {
            fn id(&self) -> crate::abilities::AbilityId {
                crate::abilities::AbilityId::new(Self::ID)
            }
            fn display_name(&self) -> &'static str {
                #name_lit
            }
            fn tags(&self) -> &'static [crate::abilities::AbilityTag] {
                &[#(crate::abilities::AbilityTag::#tags),*]
            }
            fn geometry(&self) -> crate::abilities::AbilityGeometry {
                #geometry_tokens
            }
            fn base_params(&self) -> crate::abilities::AbilityParams {
                crate::abilities::AbilityParams {
                    power: #power,
                    area: #area,
                    range: #range,
                    cast_time: #cast_time,
                    cooldown: #cooldown,
                    energy_cost: #energy_cost,
                }
            }
            fn animation(&self) -> &'static str {
                #animation
            }
            fn impact_vfx(&self) -> &'static str {
                #impact_vfx
            }
            #impact_delay_method
            #stun_seconds_method
        }

        impl #name {
            /// Registers this ability in the global registry. Generated by `#[base_ability(...)]`.
            pub fn register(registry: &mut crate::abilities::BaseAbilityRegistry) {
                registry.register(std::sync::Arc::new(#name));
            }
        }
    };

    TokenStream::from(expanded)
}

// ============================================================================
// #[essence(...)] / #[modifier(...)] / #[ancient_word(...)]
// ============================================================================
//
// All three share the same shape: metadata generated from literals, the
// actual effect logic delegated to a hand-written `*Effect` trait impl
// (`EssenceEffect`/`ModifierEffect`/`AncientWordEffect`) — see the module doc
// of `crates/shared/src/abilities/essence.rs` for why (an `impl` block can't
// be split in two, and the effect logic varies too much per Glifo to be a
// macro literal).

struct EssenceDef {
    id: LitStr,
    name: LitStr,
    rune_cost: LitInt,
    targets: Ident,
    color: (LitFloat, LitFloat, LitFloat),
}

fn parse_color_triple(input: ParseStream) -> syn::Result<(LitFloat, LitFloat, LitFloat)> {
    let content;
    parenthesized!(content in input);
    let values: Punctuated<LitFloat, Token![,]> = Punctuated::parse_terminated(&content)?;
    let values: Vec<_> = values.into_iter().collect();
    match values.as_slice() {
        [r, g, b] => Ok((r.clone(), g.clone(), b.clone())),
        _ => Err(syn::Error::new(
            content.span(),
            "color = (...) requires exactly 3 float components (r, g, b)",
        )),
    }
}

impl Parse for EssenceDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut id = None;
        let mut name = None;
        let mut rune_cost = None;
        let mut targets = None;
        let mut color = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "id" => id = Some(input.parse::<LitStr>()?),
                "name" => name = Some(input.parse::<LitStr>()?),
                "rune_cost" => rune_cost = Some(input.parse::<LitInt>()?),
                "targets" => targets = Some(input.parse::<Ident>()?),
                "color" => color = Some(parse_color_triple(input)?),
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!("unknown key `{other}` in #[essence(...)] (expected id, name, rune_cost, targets, color)"),
                    ))
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(Self {
            id: id.ok_or_else(|| input.error("#[essence(...)] requires `id = \"...\"`"))?,
            name: name.ok_or_else(|| input.error("#[essence(...)] requires `name = \"...\"`"))?,
            rune_cost: rune_cost.ok_or_else(|| input.error("#[essence(...)] requires `rune_cost = ...`"))?,
            targets: targets
                .ok_or_else(|| input.error("#[essence(...)] requires `targets = allies | enemies`"))?,
            color: color.ok_or_else(|| input.error("#[essence(...)] requires `color = (r, g, b)`"))?,
        })
    }
}

#[proc_macro_attribute]
pub fn essence(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    if let Err(err) = require_unit_struct(&input, "essence") {
        return err;
    }
    let def = parse_macro_input!(attr as EssenceDef);
    let name = &input.ident;

    let id_lit = &def.id;
    let name_lit = &def.name;
    let rune_cost = &def.rune_cost;
    let (r, g, b) = &def.color;

    // §13 of the design: no dedicated "who" Glyph — allies/enemies is a
    // built-in rule of the Essence, expressed with the caster-relative
    // `AoeTargeting` the engine already has (mirrors HealingCircle/Meteorite).
    let targeting_tokens = match def.targets.to_string().as_str() {
        "allies" => quote! { crate::spells::context::AoeTargeting::CasterOnly },
        "enemies" => quote! { crate::spells::context::AoeTargeting::ExcludeCaster },
        other => {
            return syn::Error::new_spanned(
                &def.targets,
                format!("unknown `targets = {other}` in #[essence(...)] (expected allies or enemies)"),
            )
            .to_compile_error()
            .into();
        }
    };

    let expanded = quote! {
        #input

        impl #name {
            pub const ID: &'static str = #id_lit;
        }

        impl crate::abilities::Essence for #name {
            fn id(&self) -> crate::abilities::EssenceId {
                crate::abilities::EssenceId::new(Self::ID)
            }
            fn display_name(&self) -> &'static str {
                #name_lit
            }
            fn rune_cost(&self) -> u32 {
                #rune_cost
            }
            fn default_targeting(&self) -> crate::spells::context::AoeTargeting {
                #targeting_tokens
            }
            fn visual_theme(&self) -> crate::abilities::EssenceVisualTheme {
                crate::abilities::EssenceVisualTheme {
                    color: bevy::prelude::Color::srgb(#r, #g, #b),
                }
            }
            fn manifest(
                &self,
                ability: &dyn crate::abilities::BaseAbility,
                params: &crate::abilities::AbilityParams,
                ctx: &mut crate::spells::context::SpellCastContext,
            ) {
                <Self as crate::abilities::EssenceEffect>::manifest(self, ability, params, ctx)
            }
        }

        impl #name {
            /// Registers this Essenza in the global registry. Generated by `#[essence(...)]`.
            pub fn register(registry: &mut crate::abilities::EssenceRegistry) {
                registry.register(std::sync::Arc::new(#name));
            }
        }
    };

    TokenStream::from(expanded)
}

struct ModifierDef {
    id: LitStr,
    name: LitStr,
    requires_tag: Ident,
    rune_cost: LitInt,
}

impl Parse for ModifierDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut id = None;
        let mut name = None;
        let mut requires_tag = None;
        let mut rune_cost = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "id" => id = Some(input.parse::<LitStr>()?),
                "name" => name = Some(input.parse::<LitStr>()?),
                "requires_tag" => requires_tag = Some(input.parse::<Ident>()?),
                "rune_cost" => rune_cost = Some(input.parse::<LitInt>()?),
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!("unknown key `{other}` in #[modifier(...)] (expected id, name, requires_tag, rune_cost)"),
                    ))
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(Self {
            id: id.ok_or_else(|| input.error("#[modifier(...)] requires `id = \"...\"`"))?,
            name: name.ok_or_else(|| input.error("#[modifier(...)] requires `name = \"...\"`"))?,
            requires_tag: requires_tag
                .ok_or_else(|| input.error("#[modifier(...)] requires `requires_tag = ...` (an AbilityTag)"))?,
            rune_cost: rune_cost.ok_or_else(|| input.error("#[modifier(...)] requires `rune_cost = ...`"))?,
        })
    }
}

#[proc_macro_attribute]
pub fn modifier(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    if let Err(err) = require_unit_struct(&input, "modifier") {
        return err;
    }
    let def = parse_macro_input!(attr as ModifierDef);
    let name = &input.ident;

    let id_lit = &def.id;
    let name_lit = &def.name;
    let requires_tag = &def.requires_tag;
    let rune_cost = &def.rune_cost;

    let expanded = quote! {
        #input

        impl #name {
            pub const ID: &'static str = #id_lit;
        }

        impl crate::abilities::Modifier for #name {
            fn id(&self) -> crate::abilities::ModifierId {
                crate::abilities::ModifierId::new(Self::ID)
            }
            fn display_name(&self) -> &'static str {
                #name_lit
            }
            fn required_tag(&self) -> crate::abilities::AbilityTag {
                crate::abilities::AbilityTag::#requires_tag
            }
            fn rune_cost(&self) -> u32 {
                #rune_cost
            }
            fn transform(&self, params: &mut crate::abilities::AbilityParams) {
                <Self as crate::abilities::ModifierEffect>::transform(self, params)
            }
        }

        impl #name {
            /// Registers this Modificatore in the global registry. Generated by `#[modifier(...)]`.
            pub fn register(registry: &mut crate::abilities::ModifierRegistry) {
                registry.register(std::sync::Arc::new(#name));
            }
        }
    };

    TokenStream::from(expanded)
}

struct AncientWordDef {
    id: LitStr,
    name: LitStr,
    requires_tag: Ident,
    rune_cost: LitInt,
}

impl Parse for AncientWordDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut id = None;
        let mut name = None;
        let mut requires_tag = None;
        let mut rune_cost = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "id" => id = Some(input.parse::<LitStr>()?),
                "name" => name = Some(input.parse::<LitStr>()?),
                "requires_tag" => requires_tag = Some(input.parse::<Ident>()?),
                "rune_cost" => rune_cost = Some(input.parse::<LitInt>()?),
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!("unknown key `{other}` in #[ancient_word(...)] (expected id, name, requires_tag, rune_cost)"),
                    ))
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(Self {
            id: id.ok_or_else(|| input.error("#[ancient_word(...)] requires `id = \"...\"`"))?,
            name: name.ok_or_else(|| input.error("#[ancient_word(...)] requires `name = \"...\"`"))?,
            requires_tag: requires_tag
                .ok_or_else(|| input.error("#[ancient_word(...)] requires `requires_tag = ...` (an AbilityTag)"))?,
            rune_cost: rune_cost.ok_or_else(|| input.error("#[ancient_word(...)] requires `rune_cost = ...`"))?,
        })
    }
}

#[proc_macro_attribute]
pub fn ancient_word(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    if let Err(err) = require_unit_struct(&input, "ancient_word") {
        return err;
    }
    let def = parse_macro_input!(attr as AncientWordDef);
    let name = &input.ident;

    let id_lit = &def.id;
    let name_lit = &def.name;
    let requires_tag = &def.requires_tag;
    let rune_cost = &def.rune_cost;

    let expanded = quote! {
        #input

        impl #name {
            pub const ID: &'static str = #id_lit;
        }

        impl crate::abilities::AncientWord for #name {
            fn id(&self) -> crate::abilities::AncientWordId {
                crate::abilities::AncientWordId::new(Self::ID)
            }
            fn display_name(&self) -> &'static str {
                #name_lit
            }
            fn required_tag(&self) -> crate::abilities::AbilityTag {
                crate::abilities::AbilityTag::#requires_tag
            }
            fn rune_cost(&self) -> u32 {
                #rune_cost
            }
            fn post_process(
                &self,
                ability: &dyn crate::abilities::BaseAbility,
                params: &crate::abilities::AbilityParams,
                ctx: &mut crate::spells::context::SpellCastContext,
            ) {
                <Self as crate::abilities::AncientWordEffect>::post_process(self, ability, params, ctx)
            }
        }

        impl #name {
            /// Registers this Parola Antica in the global registry. Generated by `#[ancient_word(...)]`.
            pub fn register(registry: &mut crate::abilities::AncientWordRegistry) {
                registry.register(std::sync::Arc::new(#name));
            }
        }
    };

    TokenStream::from(expanded)
}
