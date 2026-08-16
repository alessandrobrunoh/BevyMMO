# Piano di Implementazione: Correzione dell'Altezza di Lancio delle Spell su Terreni Elevati

## Descrizione del Problema
Attualmente le spell (e le abilità Eidolon) non vengono posizionate/lanciate alla corretta quota di altezza del personaggio o del punto sul terreno mirato.
Se il personaggio si trova su una montagna o su un terreno sopraelevato (o mira a un punto sopraelevato):
1. **Raycasting del cursore (`cursor_ground_point`)**: la funzione effettua l'intersezione del raggio della camera esclusivamente con il piano orizzontale fisso `Y = 0` (livello del mare) forzando `y = 0.0`, ignorando i dati del terreno (`ClientSurfaceQuery` / `SurfaceQuery`), al contrario del movimento del personaggio che usa `resolve_ray_to_ground`.
2. **Clamping della gittata nel domain (`base_ability.rs`, `meteorite/definition.rs`, `stun_field/definition.rs`)**: `clamp_to_range` azzera la coordinata `Y` impostando esplicitamente `flat_target = Vec3::new(target.x, 0.0, target.z)`, portando il centro d'impatto a quota 0 anche quando il target ha una quota `Y` valida.
3. **Anteprima di mira e Visual Effects (`aim_preview.rs`, `eidolon_effects.rs`, `meteorite`, `healing_circle`, `stun_field`)**: leggono il centro d'impatto o il punto di mira generato con quota `Y = 0`, disegnando cerchi, indicatori a terra, rocce cadenti ed effetti sotto la montagna invece che sulla superficie.

---

## User Review Required
> [!IMPORTANT]
> - **Integrazione con `ClientSurfaceQuery`**: il raycasting di mira del cursore (`cursor_ground_point`) utilizzerà `bevymmo_shared::movement::resolve_ray_to_ground` campionando le altezze effettive della mesh del terreno, mantenendo un fallback elegante a `Y = 0` qualora le superfici non siano ancora caricate.
> - **Preservazione dell'Altezza nelle Geometrie Domain**: `clamp_to_range` in `base_ability.rs` e nelle spell AoE preserverà la quota `Y` del target (o interpolerà linearmente l'altezza tra caster e target in caso di clamp oltre gittata massima), garantendo coerenza sia client-side (anteprima e visual) sia server-side.

---

## Modifiche Proposte

### 1. Presentation: Raycast del Cursore e Input

#### [MODIFY] [`crates/presentation/src/spells/cursor.rs`](file:///Users/tacosalfornoh/Coding/Rust/BevyMMO/crates/presentation/src/spells/cursor.rs)
- Aggiornare `cursor_ground_point` per accettare opzionalmente `surface_query: Option<&ClientSurfaceQuery>`.
- Se disponibile, usare `resolve_ray_to_ground(ray.origin, *ray.direction, sq, 100.0, 0.5)` per determinare il punto 3D reale sulla superficie del terreno.
- Mantenere il fallback al piano orizzontale se non c'è una superficie disponibile.

```rust
pub fn cursor_ground_point(
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    surface_query: Option<&ClientSurfaceQuery>,
) -> Option<Vec3> {
    let cursor_position = windows.single().ok()?.cursor_position()?;
    let (camera, camera_transform) = cameras.iter().next()?;
    let ray = camera
        .viewport_to_world(camera_transform, cursor_position)
        .ok()?;

    surface_query
        .and_then(|sq| sq.0.as_ref())
        .and_then(|sq| resolve_ray_to_ground(ray.origin, *ray.direction, sq, 100.0, 0.5))
        .or_else(|| {
            let t = -ray.origin.y / ray.direction.y;
            t.is_finite().then(|| ray.origin + *ray.direction * t)
        })
}
```

#### [MODIFY] [`crates/presentation/src/spells/input.rs`](file:///Users/tacosalfornoh/Coding/Rust/BevyMMO/crates/presentation/src/spells/input.rs)
- Aggiungere `surface_query: Option<Res<ClientSurfaceQuery>>` al sistema `cast_abilities_on_key`.
- Invocare `cursor_ground_point(&windows, &cameras, surface_query.as_deref())`.
- In questo modo sia `aim.ground_point` (usato dall'anteprima) sia `target_position` (inviato al server con `eidolon_cast`) conterranno l'altezza corretta.

---

### 2. Domain: Calcolo delle Aree d'Impatto e Clamping Gittata

#### [MODIFY] [`crates/domain/src/abilities/base_ability.rs`](file:///Users/tacosalfornoh/Coding/Rust/BevyMMO/crates/domain/src/abilities/base_ability.rs)
- Modificare `clamp_to_range` per non distruggere `target.y`:
  - Se entro gittata (`distance <= range` o `range <= 0.0`), restituire `target` (mantenendo `target.y`).
  - Se oltre gittata, calcolare la direzione orizzontale e interpolare la quota `Y`:
  ```rust
  fn clamp_to_range(origin: Vec3, target: Vec3, range: f32) -> Vec3 {
      if range <= 0.0 {
          return target;
      }
      let offset = flat_offset(origin, target);
      let distance = offset.length();
      if distance <= range {
          target
      } else {
          let direction = offset / distance;
          Vec3::new(
              origin.x + direction.x * range,
              origin.y + (target.y - origin.y) * (range / distance),
              origin.z + direction.z * range,
          )
      }
  }
  ```

#### [MODIFY] [`crates/domain/src/spells_impl/meteorite/definition.rs`](file:///Users/tacosalfornoh/Coding/Rust/BevyMMO/crates/domain/src/spells_impl/meteorite/definition.rs)
- Aggiornare `MeteoriteSpell::clamp_target_to_range` per preservare `target.y` invece di azzerarlo con `Vec3::new(target.x, 0.0, target.z)`.

#### [MODIFY] [`crates/domain/src/spells_impl/stun_field/definition.rs`](file:///Users/tacosalfornoh/Coding/Rust/BevyMMO/crates/domain/src/spells_impl/stun_field/definition.rs)
- Aggiornare `StunFieldSpell::clamp_target_to_range` per preservare `target.y` invece di azzerarlo con `Vec3::new(target.x, 0.0, target.z)`.

---

### 3. Presentation & Spells: Rendering ed Effetti Visivi

#### [VERIFY & ADJUST] [`crates/presentation/src/spells/aim_preview.rs`](file:///Users/tacosalfornoh/Coding/Rust/BevyMMO/crates/presentation/src/spells/aim_preview.rs)
- Verificare che `draw_ability_aim_preview` disegni i gizmos a `center + Vec3::Y * GROUND_OFFSET` con `center` avente la quota corretta di montagna/terreno.

#### [VERIFY & ADJUST] [`crates/presentation/src/spells/eidolon_effects.rs`](file:///Users/tacosalfornoh/Coding/Rust/BevyMMO/crates/presentation/src/spells/eidolon_effects.rs)
- Verificare che `spawn_ground_ring`, `spawn_falling_rock` e `spawn_burst` utilizzino `effect.start` e `effect.end` alle quote corrette (la roccia cadrà partendo da `target.y + ROCK_START_HEIGHT` fino a `target.y` sul terreno della montagna).

---

## Verification Plan

### Automated Tests
- Eseguire i test unitari sull'intero workspace:
  ```bash
  cargo test --workspace
  ```
- Aggiungere nuovi test unitari:
  1. `clamp_to_range_preserves_target_height_when_in_range` in `crates/domain/src/abilities/base_ability.rs`.
  2. `clamp_to_range_interpolates_height_when_clamped` in `crates/domain/src/abilities/base_ability.rs`.
  3. `meteorite_clamp_preserves_elevation` in `crates/domain/src/spells_impl/meteorite/definition.rs`.
  4. `stun_field_clamp_preserves_elevation` in `crates/domain/src/spells_impl/stun_field/definition.rs`.
  5. Test per `cursor_ground_point` in `crates/presentation/src/spells/cursor.rs`.

### Lint & Clippy
- Eseguire clippy:
  ```bash
  cargo clippy --workspace --all-targets -- -D warnings
  ```

### Manual Verification
- Avviare il gioco con mappa contenente rilievi montuosi (`cargo run -- client`).
- Salire su un'altura/montagna e mirare con un'abilità/spell (es. Meteorite, Arcane Seal, Stun Field, Healing Circle, Ray of Light).
- Verificare che l'anteprima (cerchio di mira) si disegni esattamente sulla superficie della montagna e non sotto.
- Lanciare l'abilità e verificare che gli effetti grafici (particelle, anello di impatto, roccia che cade) si manifestino all'altezza del personaggio sulla montagna.
