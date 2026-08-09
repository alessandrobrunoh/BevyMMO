# Piano: Audit e Fix della Pipeline delle Altezze

**Branch suggerito**: `fix/height-system-audit`
**Status**: Draft — da approvare prima dell'implementazione
**Prerequisiti**: nessuno (tutti i fix sono indipendenti tra loro)

---

## Contesto

Questo piano documenta i problemi trovati durante un audit completo della pipeline
di gestione delle altezze, dalla scultura Blender fino al movimento del player.
I piani esistenti `scale-and-height-system.md` e `blender-authored-walkable-world.md`
descrivono il design architetturale: questo piano e' focalizzato sui **bug e lacune
concrete** nel codice attualmente esistente.

### Pipeline analizzata

```
Blender sculpting (.blend)
    -> esporta
.glb + .world.json (scritto a mano)
    -> load_map_auto()
MapManifest { surfaces: Vec<WalkableSurface> }
    -> SurfaceQuery::from_manifest()
SurfaceQuery { surfaces }
    -> step_on_terrain() ogni FixedUpdate
TerrainStep::Moved(Vec3) -> Position.y aggiornato
```

### Cosa funziona bene (non toccare)

- `step_on_terrain()` - algoritmo anti-cliff, sliding, recovery
- `resolve_triangle_mesh()` - barycentric interpolation per mesh sculpted
- Server/Client parity - stesso codice su entrambi i lati
- `resolve_ray_to_ground()` - binary search per click su terreno irregolare
- `ground_at_reachable()` - asimmetria up/down corretta

---

## Fix 1: `snap_to_ground` usa `ground_at` invece di `ground_at_reachable`

### Problema

```rust
// movement.rs
pub fn snap_to_ground(position: &mut Vec3, surface_query: &SurfaceQuery) -> bool {
    if let Some(contact) = surface_query.ground_at(position.x, position.z) {
        // ground_at restituisce la superficie PIU' ALTA in quel X/Z.
        // Se ci sono due superfici sovrapposte (ground + piattaforma sopra),
        // un player a terra viene snappato alla piattaforma.
```

`ground_at` usa `highest-wins` senza filtro di raggiungibilita'. Con superfici
sovrapposte (piattaforma sopra al ground), un player a terra che viene snappato
durante Blocked/NoSurface viene teletrasportato sulla piattaforma sopra.

### Fix

```diff
// crates/shared/src/movement.rs

 pub fn snap_to_ground(position: &mut Vec3, surface_query: &SurfaceQuery) -> bool {
-    if let Some(contact) = surface_query.ground_at(position.x, position.z) {
-        if (contact.height - position.y).abs() > 0.001 {
-            position.y = contact.height;
-            return true;
-        }
+    // Usa reachable per non saltare su superfici sovrapposte irraggiungibili.
+    // Budget generoso (2.0) per recovery dopo spawn/teleport/knockback.
+    const SNAP_BUDGET: f32 = 2.0;
+    if let Some(contact) =
+        surface_query.ground_at_reachable(position.x, position.z, position.y, SNAP_BUDGET)
+    {
+        if (contact.height - position.y).abs() > 0.001 {
+            position.y = contact.height;
+            return true;
+        }
     }
     false
 }
```

> **Nota**: il budget `2.0` e' volutamente piu' grande di `max_step_height` (0.45) perche'
> `snap_to_ground` e' la recovery path (spawn, teleport, knockback). Un budget troppo piccolo
> lascerebbe il player stranded. Per il normale step-by-step usa gia' `ground_at_reachable`
> con `max_step_height` tramite `try_step()`.

### File da modificare

- `crates/shared/src/movement.rs` — funzione `snap_to_ground`

### Test da aggiungere

```rust
#[test]
fn test_snap_to_ground_does_not_jump_to_upper_platform() {
    // Due superfici sovrapposte: ground a y=0, piattaforma a y=5.
    // Player a y=0 -> snap deve rimanere a y=0, non saltare a y=5.
    let query = SurfaceQuery::from_manifest(&create_overlapping_surfaces_manifest());
    let mut pos = Vec3::new(0.0, 0.0, 0.0);
    snap_to_ground(&mut pos, &query);
    assert_eq!(pos.y, 0.0, "snap non deve teletrasportare alla piattaforma superiore");
}

#[test]
fn test_snap_to_ground_recovers_stranded_player() {
    // Player a y=10 (fuori da ogni superficie) -> deve snapparsi alla piu' alta raggiungibile.
    let query = SurfaceQuery::from_manifest(&create_overlapping_surfaces_manifest());
    let mut pos = Vec3::new(0.0, 10.0, 0.0);
    snap_to_ground(&mut pos, &query);
    assert!(pos.y <= 5.0 + 0.001, "snap deve portare il player giu' sulla superficie");
}
```

---

## Fix 2: Heightfield ignora la normale della superficie (TODO non completato)

### Problema

```rust
// collision.rs
SurfaceKind::Mesh => {
    if let Some(ref heightfield) = surface.heightfield {
        heightfield.sample_height(x, z).map(|height| {
            // TODO: Calculate proper surface normal from heightfield gradient
            GroundContact::flat(height)  // <- normale sempre (0,1,0)!
        })
    }
}
```

Il metodo `sample_normal()` esiste gia' in `HeightfieldData` ma non viene mai chiamato.
Conseguenza: su superfici heightfield (es. rolling hills), `max_slope_deg` non filtra
mai nulla perche' la normale e' sempre orizzontale.

### Fix

```diff
// crates/shared/src/world/collision.rs

 SurfaceKind::Mesh => {
     if let Some(ref mesh) = surface.walkable_mesh {
         return self.resolve_triangle_mesh(mesh, surface, x, z);
     }
     if let Some(ref heightfield) = surface.heightfield {
-        heightfield.sample_height(x, z).map(|height| {
-            // TODO: Calculate proper surface normal from heightfield gradient
-            // For now, use flat normal as approximation
-            GroundContact::flat(height)
-        })
+        let Some(height) = heightfield.sample_height(x, z) else {
+            return None;
+        };
+        let normal = heightfield
+            .sample_normal(x, z)
+            .unwrap_or([0.0, 1.0, 0.0]);
+        // Rispetta max_slope_deg se definito per questa superficie
+        if let Some(max_slope) = surface.max_slope_deg {
+            let min_normal_y = max_slope.to_radians().cos();
+            if normal[1] < min_normal_y {
+                return None; // Pendenza troppo ripida
+            }
+        }
+        Some(GroundContact::new(height, normal))
     } else {
         None
     }
 }
```

### File da modificare

- `crates/shared/src/world/collision.rs` — metodo `resolve_surface`, branch `SurfaceKind::Mesh`

### Test da aggiungere

```rust
#[test]
fn test_heightfield_slope_filter_on_mesh_surface() {
    // Heightfield con pendenza di ~63 gradi (rise/run = 2.0).
    // Con max_slope_deg = 45, il punto deve essere rifiutato.
    let bounds = SurfaceBounds { min_x: 0.0, max_x: 1.0, min_z: 0.0, max_z: 1.0 };
    // heights: [0, 0, 2, 2] -> pendenza brusca lungo X
    let hf = HeightfieldData::new(1, bounds, vec![0.0, 0.0, 2.0, 2.0]);
    let surface = WalkableSurface {
        kind: SurfaceKind::Mesh,
        heightfield: Some(hf),
        max_slope_deg: Some(45.0),
        ..minimal_surface("slope_test")
    };
    let manifest = manifest_with_surface(surface);
    let query = SurfaceQuery::from_manifest(&manifest);
    // Punto a meta' della pendenza ripida
    let result = query.ground_at(0.5, 0.5);
    assert!(result.is_none(), "pendenza > 45 gradi deve essere rifiutata");
}
```

---

## Fix 3: `CollisionGrid::is_blocked()` ignora la coordinata Y

### Problema

```rust
// collision.rs
pub fn is_blocked(&self, point: [f32; 3], radius: f32) -> bool {
    self.obstacles.iter().any(|obstacle| {
        let closest_x = point[0].clamp(obstacle.min[0], obstacle.max[0]);
        let closest_z = point[2].clamp(obstacle.min[2], obstacle.max[2]);
        // Y intentionally ignored while terrain is flat.  <- non e' piu' flat!
        let dx = point[0] - closest_x;
        let dz = point[2] - closest_z;
        dx * dx + dz * dz <= radius * radius
    })
}
```

Con terreno multi-livello, una roccia a Y=0 blocca un player a Y=5 che si trova
sopra di essa su una piattaforma. Il commento "while terrain is flat" non e' piu' valido.

### Fix

```diff
// crates/shared/src/world/collision.rs

 pub fn is_blocked(&self, point: [f32; 3], radius: f32) -> bool {
     self.obstacles.iter().any(|obstacle| {
+        // Vertical overlap: il point[1] e' la base del player (piedi).
+        // Usiamo una altezza fissa conservativa; un blocker che non
+        // si sovrappone verticalmente al player non puo' bloccarlo.
+        const PLAYER_HEIGHT: f32 = 1.8;
+        let player_top = point[1] + PLAYER_HEIGHT;
+        let vertical_overlap = point[1] < obstacle.max[1] && player_top > obstacle.min[1];
+        if !vertical_overlap {
+            return false;
+        }
+
         let closest_x = point[0].clamp(obstacle.min[0], obstacle.max[0]);
         let closest_z = point[2].clamp(obstacle.min[2], obstacle.max[2]);
         let dx = point[0] - closest_x;
         let dz = point[2] - closest_z;
         dx * dx + dz * dz <= radius * radius
     })
 }
```

> **Nota design**: `PLAYER_HEIGHT` potrebbe diventare un parametro derivato da
> `WorldMetrics.player_height` in futuro. Per ora evita di cambiare la firma pubblica
> di `is_blocked`, che e' usata in molti punti.

### File da modificare

- `crates/shared/src/world/collision.rs` — metodo `is_blocked`
- Aggiornare il commento nella definizione di `CollisionGrid` per rimuovere "while terrain is flat"

### Test da aggiungere

```rust
#[test]
fn test_blocker_ground_does_not_block_player_on_platform() {
    // Roccia: min_y=0, max_y=3. Player a y=5 (su piattaforma) -> non bloccato.
    // Player a y=0 (a terra) -> bloccato.
    let grid = /* CollisionGrid con blocker [4..6, 0..3, 4..6] */;

    assert!(
        !grid.is_blocked([5.0, 5.0, 5.0], 0.5),
        "blocker a terra non deve bloccare player su piattaforma sopra"
    );
    assert!(
        grid.is_blocked([5.0, 0.0, 5.0], 0.5),
        "blocker a terra deve bloccare player a terra"
    );
}
```

---

## Fix 4: Validazione convenzione assi nel `walkable_mesh`

### Problema

I vertici in `WalkableMeshData` devono essere `[x, y, z]` world-space Y-up.
In `resolve_triangle_mesh()`:

```rust
// v[0]=X, v[1]=Y (altezza), v[2]=Z usati per test 2D e interpolazione altezza
```

Nel JSON del ramp `walkable_world_map.world.json`:
```json
{ "vertices": [
    [-6.4509, 0.0, 0.0],
    [ 2.2587, 0.0, 0.0],
    [ 2.2587, 8.0, 4.0],   // y=8 sembra la profondita' lungo Z, non l'altezza
    [-6.4509, 8.0, 4.0]
]}
```

Se il formato e' `[x, y_altezza, z]`, allora `y=8` per il bordo alto di una rampa
che sale di 4 unita' sembra errato. E' necessario chiarire il formato usato dallo
script che ha generato il JSON, e aggiungere validazione nel loader che avvisi
quando i triangoli sono degeneri nel piano XZ (segnale di assi scambiati).

### Fix: aggiungere validazione triangoli nel loader

```diff
// crates/shared/src/world/loader.rs

+/// Valida un WalkableMeshData: indici in bounds e triangoli non degeneri nel piano XZ.
+fn validate_walkable_mesh(mesh: &WalkableMeshData, id: &str) -> Vec<ValidationIssue> {
+    let mut issues = Vec::new();
+    if mesh.indices.len() % 3 != 0 {
+        issues.push(ValidationIssue::new(format!(
+            "surface '{id}': walkable_mesh ha {} indici, attesi multiplo di 3",
+            mesh.indices.len()
+        )));
+    }
+    for &idx in &mesh.indices {
+        if idx as usize >= mesh.vertices.len() {
+            issues.push(ValidationIssue::new(format!(
+                "surface '{id}': indice {idx} fuori bounds (nvertici={})",
+                mesh.vertices.len()
+            )));
+        }
+    }
+    let tri_count = mesh.indices.len() / 3;
+    for tri in 0..tri_count {
+        let i0 = mesh.indices[tri * 3] as usize;
+        let i1 = mesh.indices[tri * 3 + 1] as usize;
+        let i2 = mesh.indices[tri * 3 + 2] as usize;
+        if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
+            continue; // gia' segnalato
+        }
+        let v0 = mesh.vertices[i0];
+        let v1 = mesh.vertices[i1];
+        let v2 = mesh.vertices[i2];
+        // Area nel piano X/Z (cio' che il pathfinder usa per point-in-triangle)
+        let ax = v1[0] - v0[0]; let az = v1[2] - v0[2];
+        let bx = v2[0] - v0[0]; let bz = v2[2] - v0[2];
+        let area_xz = (ax * bz - az * bx).abs();
+        if area_xz < 1e-4 {
+            issues.push(ValidationIssue::new(format!(
+                "surface '{id}': triangolo {tri} degenere nel piano XZ (area={area_xz:.2e}). \
+                 Possibile confusione assi Y/Z nel vertex format [x, y_altezza, z_mondo].",
+            )));
+        }
+    }
+    issues
+}
```

Integrare in `validate_structure()`:

```diff
 // in validate_structure()
+    for surface in &manifest.surfaces {
+        if let Some(ref mesh) = surface.walkable_mesh {
+            issues.extend(validate_walkable_mesh(mesh, &surface.id));
+        }
+    }
```

### File da modificare

- `crates/shared/src/world/loader.rs` — aggiungere `validate_walkable_mesh`
  e chiamarla in `validate_structure`

---

## Fix 5: Documentare il workflow Blender -> world.json

### Problema

Non esiste nessuna documentazione che spieghi come generare il `.world.json`
da una scultura Blender. Niente script, niente convenzioni documentate, niente
guida. Per usare terreno sculpted devi capire il formato a ritroso dal JSON.

### Fix: creare docs/blender-export-workflow.md

Il documento deve coprire:

**1. Convenzione nomi oggetti in Blender**
- `WALKABLE_*` -> superfici percorribili (mesh triangolata o flat)
- `RAMP_*` -> rampe con walkable_mesh
- `BLOCKER_*` -> colliders che bloccano il movimento

**2. Formato vertex nel walkable_mesh**
```
Vertex format: [x, y, z] world-space Y-up (standard glTF)
  x = coordinata X nel mondo
  y = ALTEZZA (coordinata verticale, su in gioco)
  z = coordinata Z nel mondo (profondita')

Blender usa Z-up. Dopo esportazione glTF la conversione e':
  Blender (x, y, z) -> game [x, z, -y]
  cioe': la Y di Blender diventa -Z in game, la Z di Blender diventa Y in game.

Per ottenere il vertex [x, y_altezza, z_mondo] dallo spazio Blender:
  game_x = blender_x
  game_y = blender_z   (altezza in Blender = Z)
  game_z = -blender_y  (profondita' in Blender = -Y)
```

**3. Script Python Blender per estrarre walkable_mesh**

```python
import bpy, json, bmesh

obj = bpy.context.active_object
bm = bmesh.new()
bm.from_mesh(obj.data)
bmesh.ops.triangulate(bm, faces=bm.faces)

# Blender (x, y, z) -> game [x, z, -y]
vertices = [[v.co.x, v.co.z, -v.co.y] for v in bm.verts]
indices = [l.vert.index for f in bm.faces for l in f.loops]
bm.free()

print(json.dumps({"vertices": vertices, "indices": indices}, indent=2))
```

### File da creare

- `docs/blender-export-workflow.md`

---

## Ordine di Implementazione Consigliato

| # | Fix | Impatto | Rischio | Effort |
|---|-----|---------|---------|--------|
| 1 | Fix 4: Validazione mesh nel loader | Diagnostica, nessun rischio | Basso | ~1h |
| 2 | Fix 2: Normale heightfield | Correttezza fisica rolling hills | Basso | ~1h |
| 3 | Fix 1: snap_to_ground reachable | Bug con superfici sovrapposte | Medio | ~1h |
| 4 | Fix 3: CollisionGrid Y-aware | Multi-livello corretto | Medio | ~2h |
| 5 | Fix 5: Docs Blender workflow | Usabilita' per il designer | Nessuno | ~2h |

---

## Piano di Verifica

### Test automatici

```bash
cargo test -p bevymmo-shared -- world::collision
cargo test -p bevymmo-shared -- movement
cargo clippy -- -D warnings
```

Test specifici che devono passare dopo ogni fix:

- **Fix 1**: `test_snap_to_ground_does_not_jump_to_upper_platform` (nuovo)
- **Fix 2**: `test_heightfield_slope_filter_on_mesh_surface` (nuovo)
- **Fix 3**: `test_blocker_ground_does_not_block_player_on_platform` (nuovo)
- **Fix 4**: `validate_structure` ritorna warning su mesh degeneri, OK su mesh valide

### Verifica manuale

```bash
cargo run -- host-client
```

Con la mappa `walkable_world_map`:

1. **Fix 1**: Player a terra sotto una piattaforma. Click su punto a terra -> player rimane a terra, non salta sulla piattaforma.
2. **Fix 2**: Log del server mostra `ground_y=None` su pendenze > 45 gradi nelle rolling hills.
3. **Fix 3**: Albero/roccia a Y=0. Player su piattaforma sopra all'albero -> cammina liberamente senza essere bloccato.
4. **Fix 4**: Editare temporaneamente un vertex del world.json con Y e Z scambiate -> il server logga un warning di triangolo degenere al caricamento.

---

## Cosa NON e' in scope

- Script Blender completo con UI/addon -> separato
- Pathfinding / navmesh -> fuori scope attuale
- Rotazione dei blocker negli obstacle AABB -> gia' fuori scope nel codice esistente
- Cambio del formato del manifest -> nessuna rottura della compatibilita'

---

## Open Questions

> [!IMPORTANT]
> **Asse Y/Z nel walkable_mesh**: i vertici del ramp in `walkable_world_map.world.json`
> sembrano avere Y e Z scambiate rispetto al formato `[x, y_altezza, z]`. Prima di
> implementare Fix 4, verificare: la rampa `ramp_ground_to_test_top` funziona
> correttamente in gioco oggi? Se si', il formato effettivo potrebbe essere
> `[x, z_mondo, y_altezza]` e sia il codice che i docs vanno allineati a quella
> convenzione. Se no, i dati JSON vanno corretti.

> [!NOTE]
> **SNAP_BUDGET in Fix 1**: `2.0` e' arbitrario. Potrebbe diventare una costante
> pubblica in `movement.rs` o derivarsi da `WorldMetrics.player_height * 2`.
> Per ora e' una costante locale per non cambiare la firma pubblica.
