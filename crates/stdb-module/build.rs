//! Pre-compiles the authored map manifests into the module binary.
//!
//! The module runs in a WASM sandbox with no filesystem, so
//! `bevymmo_shared::world::loader` — which reads `.world.json` off disk — is
//! unusable there. This build script runs on the **host**, where `std::fs` and
//! `serde_json` do exist: it parses every `assets/maps/*.world.json` into a
//! [`MapManifest`], re-emits it in a compact binary form under `OUT_DIR`, and
//! generates a tiny `maps.rs` of `include_bytes!` entries that `src/world.rs`
//! pulls in.
//!
//! # Why not `include_str!` the JSON
//!
//! `assets/maps/map_02.world.json` is 3.9 MB, ~95% of it one array of 130 321
//! heightfield floats printed as decimal text. Embedding that would mean
//! carrying ~4 MB of text in the `.wasm` *and* running a JSON parser inside the
//! module at `init`. The other three maps are under 21 KB each, so the format
//! is chosen entirely for map_02's heightfield.
//!
//! # Why postcard
//!
//! Not size: bincode 2's `standard` config varint-encodes lengths too, and over
//! all four maps it landed within 5 bytes of postcard (557 251 vs 557 246). The
//! reason is the decoder that ships *inside* the module. postcard is `no_std`
//! with an `alloc`-only feature, so the WASM side pulls in a parser and nothing
//! else; bincode's decode path wants `std` and carries error and configuration
//! machinery the module never uses. Both are non-self-describing, which is
//! where the real win over JSON comes from — no field names on the wire and no
//! text-to-float parsing at `init`.
//!
//! Postcard is **not** self-describing, so encoder and decoder must agree on
//! the exact type. [`EncodedMap`] here and its twin in `src/world.rs` are that
//! agreement: a mismatch decodes as garbage rather than erroring, which is why
//! both sides carry the same comment.
//!
//! # Failure policy
//!
//! Everything here panics on bad input. A corrupt or unparseable manifest must
//! break the build, not the running server: at runtime the module has no way to
//! recover, and a world that silently seeds nothing is far harder to diagnose
//! than a failed `spacetime build`.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use bevymmo_domain::world::MapManifest;

/// One quantised heightfield: `(index of the surface it belongs to, minimum
/// height, quantisation step, little-endian `u16` samples)`.
///
/// The samples are `Vec<u8>` rather than `Vec<u16>` on purpose: postcard
/// varint-encodes `u16`, and values spread across the full 0..=65535 range cost
/// 3 bytes each that way — worse than the 2 bytes they need. `u8` has no varint
/// form, so a byte vector is written verbatim after its length.
type EncodedHeightfield = (u32, f32, f32, Vec<u8>);

/// What one `OUT_DIR/maps/<map_id>.bin` holds: the manifest with every
/// heightfield's `heights` emptied, plus those heights quantised alongside.
///
/// Must stay structurally identical to the alias of the same name in
/// `src/world.rs`.
type EncodedMap = (MapManifest, Vec<EncodedHeightfield>);

/// Number of quantisation steps between a heightfield's min and max.
///
/// `u16` gives 65 535 steps. map_02 spans ~20 m of relief, so one step is
/// ~0.31 mm and the worst rounding error is half of that — 0.15 mm, which is
/// what a round-trip measured. That is three orders of magnitude below
/// `max_step_height` (0.45 m), and it moves the slope that
/// `HeightfieldData::sample_normal` reconstructs over its 1 m stencil by at
/// most ~0.02°, against a walkable threshold of 45-50°. Nothing a player can
/// stand on, walk up, or be blocked by changes; the array halves in size.
const QUANTISATION_LEVELS: f32 = u16::MAX as f32;

fn main() {
    let maps_dir = locate_maps_dir();
    println!("cargo:rerun-if-changed=build.rs");
    // Read through `option_env!` by `world::GM_IDENTITIES`; without this a
    // changed GM list would not reach a cached build.
    println!("cargo:rerun-if-env-changed=BEVYMMO_GM_IDENTITIES");
    println!("cargo:rerun-if-changed={}", maps_dir.display());

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("cargo always sets OUT_DIR"));
    let maps_out = out_dir.join("maps");
    fs::create_dir_all(&maps_out)
        .unwrap_or_else(|error| panic!("cannot create {}: {error}", maps_out.display()));

    // Sorted so the generated table — and therefore the `.wasm` — is
    // reproducible across machines with different directory orderings.
    let sources: BTreeSet<PathBuf> = fs::read_dir(&maps_dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", maps_dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".world.json"))
        })
        .collect();

    assert!(
        !sources.is_empty(),
        "no *.world.json manifests under {}; the module would seed an empty world",
        maps_dir.display()
    );

    let mut table = String::from(
        "// Generated by build.rs. The manifests, compiled into the module.\n\
         pub static EMBEDDED_MAPS: &[(&str, &[u8])] = &[\n",
    );

    for source in &sources {
        println!("cargo:rerun-if-changed={}", source.display());
        let map_id = compile_manifest(source, &maps_out);
        table.push_str(&format!(
            "    ({map_id:?}, include_bytes!(concat!(env!(\"OUT_DIR\"), \"/maps/{map_id}.bin\"))),\n"
        ));
    }
    table.push_str("];\n");

    let table_path = out_dir.join("maps.rs");
    fs::write(&table_path, table)
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", table_path.display()));
}

/// Parses one `.world.json`, encodes it into `maps_out`, and returns its map id.
fn compile_manifest(source: &Path, maps_out: &Path) -> String {
    let json = fs::read_to_string(source)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", source.display()));
    let mut manifest: MapManifest = serde_json::from_str(&json)
        .unwrap_or_else(|error| panic!("{} is not a MapManifest: {error}", source.display()));
    manifest
        .validate()
        .unwrap_or_else(|error| panic!("{} failed validation: {error}", source.display()));

    // The sidecar is picked by filename on the client and by `map_id` in the
    // database (`prop_override.map_id`). When the two disagree the map collides
    // against one file and renders another; the native loader only warns about
    // it, but the module has no operator watching its log, so make it fatal.
    let stem = source
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".world.json"))
        .unwrap_or_else(|| panic!("{} has no usable file stem", source.display()));
    assert_eq!(
        stem,
        manifest.map_id,
        "{} declares map_id {:?}; rename the file or fix map_id",
        source.display(),
        manifest.map_id
    );

    let mut heightfields: Vec<EncodedHeightfield> = Vec::new();
    for (index, surface) in manifest.surfaces.iter_mut().enumerate() {
        let Some(heightfield) = surface.heightfield.as_mut() else {
            continue;
        };
        heightfield.validate().unwrap_or_else(|error| {
            panic!(
                "{}: surface {:?} has an inconsistent heightfield: {error}",
                source.display(),
                surface.id
            )
        });
        if heightfield.heights.is_empty() {
            continue;
        }
        let (min, step, samples) = quantise(&heightfield.heights);
        heightfields.push((index as u32, min, step, samples));
        // Emptied here so the float array is written once, quantised, instead
        // of twice.
        heightfield.heights = Vec::new();
    }

    let map_id = manifest.map_id.clone();
    let encoded: EncodedMap = (manifest, heightfields);
    let bytes = postcard::to_allocvec(&encoded)
        .unwrap_or_else(|error| panic!("cannot encode {}: {error}", source.display()));

    let destination = maps_out.join(format!("{map_id}.bin"));
    fs::write(&destination, &bytes)
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", destination.display()));
    map_id
}

/// Maps `heights` onto `0..=u16::MAX` between their own min and max.
///
/// Returns the offset and step the module needs to reverse it. A flat
/// heightfield gets a zero step, which decodes back to a constant `min`.
fn quantise(heights: &[f32]) -> (f32, f32, Vec<u8>) {
    let min = heights.iter().copied().fold(f32::INFINITY, f32::min);
    let max = heights.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        min.is_finite() && max.is_finite(),
        "heightfield contains NaN or infinity"
    );

    let step = if max > min {
        (max - min) / QUANTISATION_LEVELS
    } else {
        0.0
    };

    let mut samples = Vec::with_capacity(heights.len() * 2);
    for height in heights {
        let level = if step > 0.0 {
            ((height - min) / step)
                .round()
                .clamp(0.0, QUANTISATION_LEVELS) as u16
        } else {
            0
        };
        samples.extend_from_slice(&level.to_le_bytes());
    }
    (min, step, samples)
}

/// Finds `assets/maps`, walking up from this crate.
///
/// Mirrors `bevymmo_shared::paths::assets_root`'s walk-up strategy rather than
/// hardcoding `../../assets`, so the module still builds if it is ever vendored
/// at a different depth.
fn locate_maps_dir() -> PathBuf {
    let mut current =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("cargo always sets this"));
    loop {
        let candidate = current.join("assets").join("maps");
        if candidate.is_dir() {
            return candidate;
        }
        if !current.pop() {
            panic!("no assets/maps directory above the module crate");
        }
    }
}
