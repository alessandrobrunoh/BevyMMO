## Obiettivo
Analizzare e valutare il sistema di gestione delle altezze (Height/Terrain) e del movimento del Player nella mappa creata tramite scultura in Blender. Il documento evidenzia i punti di forza e le criticità dell'implementazione attuale, proponendo le relative soluzioni.

## User Review Required
> [!WARNING]
> La generazione del terreno direttamente da Blender `.glb` non avviene in modo automatico nel codice Rust attuale. Il gioco utilizza un file `.world.json` generato separatamente che contiene i dati delle collisioni (`HeightfieldData` o `WalkableMeshData`). Se ci si aspetta che il file `.glb` puro di Blender sia sufficiente, sarà necessario creare uno script Python/Blender per esportare automaticamente questo `.world.json`.
>
> Inoltre, ho individuato alcuni **bug critici** nel sistema di scivolamento sui muri e nel puntamento del mouse che propongo di sistemare prima di andare in produzione.

## Analisi Attuale: Cosa è fatto BENE (Punti di forza)

1. **Prevenzione del "Teletrasporto" sui dirupi (`ground_at_reachable`)**
   L'algoritmo filtra le superfici raggiungibili in base all'altezza massima scalabile (`max_step_height`). Questo impedisce al player di "teletrasportarsi" istantaneamente in cima a una montagna o su un tetto con un singolo click.
2. **Determinismo (Nessun motore fisico pesante)**
   Il calcolo delle altezze usa matematica pura senza dipendere da motori fisici (come Rapier). Questo garantisce che la predizione del client e il server autoritativo si comportino in modo **identico**, evitando fastidiosi scatti (rubber-banding).
3. **Discesa asimmetrica e recupero**
   Il giocatore può cadere dai bordi dolcemente e c'è un sistema di `snap_to_ground` che recupera il personaggio nel caso in cui finisca leggermente "sospeso" o sprofondato a causa di arrotondamenti di calcolo.

## Analisi Attuale: Cosa è fatto MALE (Difetti e Bug)

1. **Bug: Velocità folle scivolando sui muri (Hyper-Speed Wall Sliding)**
   Se il giocatore cammina in diagonale contro un muro, l'algoritmo attuale cancella una direzione per farlo scivolare (es. blocca la Z), ma poi **rinormalizza** il vettore rimanente. Questo fa sì che il giocatore schizzi lungo il muro a una velocità inaspettata (fino a 10 volte più veloce del normale).
2. **Bug: Mirare sotto i ponti ("Highest-Wins" Raycast)**
   Quando clicchi col mouse (`resolve_ray_to_ground`), il raggio cerca il terreno calcolando l'altezza massima in quel punto. Se clicchi sotto un ponte o sotto l'ombra di un grande albero sculturato, il gioco penserà che tu abbia cliccato sopra il ponte, facendo camminare il player nel posto sbagliato.
3. **Collisioni degli ostacoli in 2D (Y-Blindness)**
   Gli ostacoli e i muri (`CollisionGrid`) sono calcolati solo in X e Z (2D). Questo significa che se cammini su un ponte alto, e sotto c'è un recinto o un prop, il giocatore sul ponte verrà bloccato da un "muro invisibile" generato dal recinto sottostante.
4. **Performance sulle Mesh di Blender (Ricerca O(N))**
   Se la mesh esportata da Blender ha molti triangoli per le montagne, il gioco esegue un test punto-triangolo iterando su **tutti** i triangoli della mappa a ogni tick. Manca una struttura di accelerazione spaziale (come un Quadtree o BVH).

---

## Proposed Changes

Di seguito le proposte per risolvere le criticità trovate.

### `crates/shared/src/movement.rs`
#### [MODIFY] movement.rs
Fix del bug di scivolamento (Wall Sliding Speed Bug) e miglioramento della gestione dello step. Non normalizzeremo il vettore di scivolamento (slide vector) a `1.0`, ma manterremo la sua magnitudine proiettata corretta.

```diff
-    let len = (dir_x * dir_x + dir_z * dir_z).sqrt();
-    if len < 1e-6 {
-        return None;
-    }
-
-    let next_x = current.x + dir_x / len * step;
-    let next_z = current.z + dir_z / len * step;

+    // Maintain component magnitude instead of re-normalizing for slides
+    let next_x = current.x + dir_x * step;
+    let next_z = current.z + dir_z * step;
```

#### [MODIFY] movement.rs
Miglioramento del `resolve_ray_to_ground` per supportare ponti e caverne. Invece di usare `surface_query.ground_at(x, z)` (che prende la superficie più alta), si deve usare una funzione che prenda la superficie più vicina al raggio che scende verso il basso, o modificare la logica di bisezione per trovare il punto di impatto corretto.

### `crates/shared/src/world/collision.rs`
#### [MODIFY] collision.rs
Aggiornamento della `CollisionGrid` per includere l'asse Y (Altezza). Questo permetterà di ignorare gli ostacoli che si trovano troppo sotto o troppo sopra il giocatore.

### Quadtree per Mesh Complesse (Futuro / Opzionale)
Se le tue mappe di Blender diventano molto grandi, suggerisco di implementare un piccolo BVH o Quadtree dentro `resolve_triangle_mesh` in `collision.rs` per evitare cali di frame rate.

## Verification Plan

### Manual Verification
1. **Test del Muro:** Camminare in diagonale contro un muro inclinato e verificare che il player scivoli fluidamente senza accelerazioni improvvise.
2. **Test del Ponte/Arco:** Costruire un ponte scolpito in Blender, cliccare sul terreno **sotto** il ponte e assicurarsi che il player cammini sotto (e non sopra).
3. **Test degli Ostacoli multi-livello:** Passare sopra un ponte mentre c'è un ostacolo al livello inferiore e verificare che il player non venga bloccato.
