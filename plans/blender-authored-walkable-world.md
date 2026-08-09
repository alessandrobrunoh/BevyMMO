# Piano: Mondo walkable authored in Blender — piattaforme, rampe curve, colline e piani

**Status**: Draft — da approvare prima dell'implementazione
**Branch suggerito**: `feat/blender-authored-walkable-world`
**Obiettivo**: sostituire l'attuale interpretazione troppo generica dei bounds/heightfield con un modello di movimento esplicito, compatibile con mappe stile Albion: superfici realmente calpestabili, rampe dritte o curve, piattaforme a quote discrete e blocker sui bordi non percorribili.

---

## 1. Risultato desiderato

Il level designer deve poter costruire in Blender una collina come questa:

```text
VISUAL:     una collina/roccia unica, anche irregolare
GAMEPLAY:   solo il percorso realmente percorribile
            ┌──────────── vetta
        ╭───╯
     ╭──╯     rampa curva / a mezzaluna
─────╯        terreno basso
BLOCKERS:     bordi rocciosi e lati non percorribili
```

Il gioco deve comportarsi così:

- il personaggio cammina solo sulle superfici gameplay autorizzate;
- può salire da una piattaforma all'altra solo passando da una rampa o scala;
- la rampa può essere dritta, curva, a S, a mezzaluna o switchback;
- ogni rampa può avere una pendenza diversa;
- una piattaforma alta non è raggiungibile semplicemente cliccando dentro il suo rettangolo;
- una collina visuale non diventa automaticamente tutta walkable;
- i bordi di roccia/cliff bloccano il movimento dove non esiste un passaggio;
- server e client usano esattamente la stessa rappresentazione gameplay.

---

## 2. Decisioni architetturali

### 2.1 La geometria visuale non è la geometria gameplay

Il `.glb` contiene principalmente la scena visuale:

- terreno dettagliato;
- roccia;
- edifici;
- decorazioni;
- modelli di scale e rampe;
- materiali e luci.

Il `.world.json` contiene la geometria semplificata per il gameplay:

- mesh walkable triangolate;
- piattaforme flat;
- ramp mesh anche curve;
- blocker con trasformazione e forma collisione;
- quote e pendenze;
- ids stabili e riferimenti tra superfici.

Il server non deve ricostruire la navigabilità guardando nomi casuali o dettagli della mesh visuale.

### 2.2 Una rampa non è solo `start -> end`

`start/end` è sufficiente solo per una rampa rettilinea molto semplice. Il modello principale sarà invece una **walkable surface mesh**:

```text
(x, z) + superficie/layer corrente -> altezza y, normale, superficie valida
```

La mesh gameplay può rappresentare:

- rampa dritta;
- rampa curva;
- mezzaluna;
- tornante;
- switchback;
- terreno collinare percorribile;
- ponte;
- scala trattata come rampa liscia;
- pianerottolo.

`TraversalData` potrà restare come metadato di collegamento, ma non deve essere l'unica fonte della geometria.

### 2.3 Ogni patch walkable deve avere forma reale

Non usare un rettangolo `bounds` come unica definizione di una rampa a mezzaluna o di una collina irregolare. Il rettangolo può essere mantenuto come broad phase, ma deve essere seguito da un test preciso sulla mesh o su una maschera walkable.

```text
bounds rettangolare = filtro veloce
mesh/mask = decisione finale
```

### 2.4 Quote discrete e salite continue

Le piattaforme possono avere quote discrete:

```text
ground             y = 0
mountain_1_top     y = 4
mountain_2_top     y = 8
castle_floor_1     y = 0
castle_floor_2     y = 3.5
castle_floor_3     y = 7
castle_floor_4     y = 10.5
```

Le rampe connettono le quote gradualmente. La quota non deve essere dedotta dalla priorità del nome: deve risultare dai vertici della mesh gameplay.

---

## 3. Struttura Blender obbligatoria

Creare una nuova mappa con queste Collection:

```text
MAP_Root
├── VISUAL
│   ├── Terrain_Visual
│   ├── Mountain_01_Visual
│   ├── Rock_01_Visual
│   ├── Castle_Visual
│   └── Props_Visual
│
├── GAMEPLAY
│   ├── WALKABLE_Ground
│   ├── WALKABLE_Mountain_01_Top
│   ├── WALKABLE_Mountain_02_Top
│   ├── WALKABLE_Hill_01_Path
│   ├── RAMP_Ground_to_Mountain_01
│   ├── RAMP_Mountain_01_Crescent
│   ├── RAMP_Mountain_01_to_02
│   ├── WALKABLE_Castle_Floor_01
│   ├── WALKABLE_Castle_Floor_02
│   └── RAMP_Castle_Floor_01_to_02
│
├── COLLISION
│   ├── BLOCKER_Mountain_01_LeftCliff
│   ├── BLOCKER_Mountain_01_RightCliff
│   ├── BLOCKER_Mountain_02_BackCliff
│   ├── BLOCKER_Castle_Floor_02_EastEdge
│   └── BLOCKER_MapBoundary_North
│
└── DEBUG
    ├── DEBUG_SurfaceLabels
    └── DEBUG_TraversalLinks
```

### Regole di separazione

- Gli oggetti in `VISUAL` non sono automaticamente walkable.
- Gli oggetti in `GAMEPLAY` non devono contenere decorazioni, pareti, underside o soffitti.
- Gli oggetti in `COLLISION` rappresentano solo volumi che impediscono il movimento.
- Gli oggetti in `DEBUG` non entrano nella build finale.
- Non duplicare l'intera montagna per ogni livello gameplay.
- Duplicare solo la porzione realmente calpestabile, semplificata e low-poly.

---

## 4. Come modellare una collina con salita parziale

### 4.1 Modello visuale

Creare una sola collina completa:

```text
VISUAL/Mountain_01_Visual
```

Questa può contenere roccia, dettagli, pareti ripide e tutte le parti non walkable.

### 4.2 Superfici gameplay

Creare separatamente:

```text
GAMEPLAY/WALKABLE_Mountain_01_Base
GAMEPLAY/RAMP_Mountain_01_Crescent
GAMEPLAY/WALKABLE_Mountain_01_Top
```

La mesh della mezzaluna deve contenere solo la striscia percorribile. Non deve coprire l'intero rettangolo della montagna.

Esempio dall'alto:

```text
roccia non walkable
████████████████████
██                ██
██    ╭───────╮   ██
██   ╱         ╰╮ ██
██  ╱  percorso  ╰██
██ ╰────────────── ██
████████████████████
```

### 4.3 Blocker

Aggiungere volumi lungo i lati dove il personaggio non può entrare:

```text
COLLISION/BLOCKER_Mountain_01_LeftCliff
COLLISION/BLOCKER_Mountain_01_RightCliff
COLLISION/BLOCKER_Mountain_01_BackCliff
```

La rampa non deve essere circondata da un unico grande box che ne chiuda l'ingresso. I blocker devono essere segmentati e lasciare libero il corridoio della rampa.

---

## 5. Come modellare rampe curve o a mezzaluna

### Workflow Blender consigliato

1. Modellare il tracciato con una `Bezier Curve` o `Path Curve`.
2. Impostare la larghezza del percorso.
3. Convertire la curva in una ribbon mesh oppure usare una mesh low-poly già modellata.
4. Proiettare/conformare la mesh alla collina visuale.
5. Controllare che la quota cresca nella direzione desiderata.
6. Eliminare triangoli invertiti, buchi e facce verticali.
7. Triangolare la mesh gameplay in modo deterministico.
8. Applicare la trasformazione world-space prima dell'export.
9. Aggiungere metadata `surface_kind = "ramp"` o `surface_kind = "walkable_mesh"`.

La forma curva non richiede un nuovo algoritmo speciale: il runtime interroga la mesh triangolata e trova l'altezza locale.

### Pendenza

Per ogni triangolo o campione calcolare la normale. Una superficie è valida se:

```text
angle(normal, Vec3::Y) <= max_walkable_slope_deg
```

Il limite globale di default può essere `45°`, mentre una rampa specifica può dichiarare un limite più basso.

La pendenza diversa tra due rampe è quindi un attributo della geometria, non una convenzione sul nome.

---

## 6. Metadata Blender

L'exporter deve leggere Custom Properties dagli oggetti gameplay. I nomi sono solo una verifica umana; le properties sono il contratto macchina.

### Properties comuni

```text
gameplay = true
id = "ramp_mountain_01_crescent"
```

### Piattaforma flat

```text
gameplay_type = "walkable_surface"
surface_kind = "flat"
surface_id = "mountain_01_top"
walkable = true
layer = "mountain_01"
```

### Mesh walkable/rampa

```text
gameplay_type = "walkable_surface"
surface_kind = "mesh"
surface_id = "ramp_mountain_01_crescent"
walkable = true
max_slope_deg = 38.0
layer = "mountain_01"
```

### Blocker

```text
gameplay_type = "blocker"
blocker_id = "mountain_01_left_cliff"
collision_kind = "box"
blocks_movement = true
```

### Collegamenti opzionali

```text
from_surface = "ground"
to_surface = "mountain_01_top"
traversal_kind = "ramp"
```

Questi collegamenti servono a validazione e debug, ma il movimento non deve dipendere solo da essi: la mesh deve essere sufficiente a determinare dove il player può stare.

---

## 7. Nuovo formato gameplay nel manifest

### Superficie

Evolvere `WalkableSurface` per supportare una rappresentazione precisa:

```rust
pub struct WalkableSurface {
    pub id: String,
    pub kind: SurfaceKind,
    pub object: Option<String>,
    pub bounds: Option<SurfaceBounds>,
    pub height: Option<f32>,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    pub max_slope_deg: Option<f32>,
    pub layer: Option<String>,
    pub mesh: Option<WalkableMeshData>,
}
```

Possibile enum:

```rust
pub enum SurfaceKind {
    Flat,
    Mesh,
}
```

Non è obbligatorio introdurre `Ramp` come kind separato: una rampa curva è semplicemente una `Mesh` walkable. Un campo `surface_role = flat | ramp | terrain | platform` può essere aggiunto per debug e validazione senza duplicare la matematica.

### Mesh gameplay

```rust
pub struct WalkableMeshData {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub bounds: SurfaceBounds,
}
```

La prima versione può usare una struttura compatta derivata, purché il server riesca a risolvere:

```text
(x, z) -> triangolo walkable -> y + normale
```

### Blocker

`BlockerData` deve contenere dati spaziali reali. Un riferimento al nome Blender non è sufficiente.

```rust
pub struct BlockerData {
    pub id: String,
    pub kind: BlockerKind,
    pub object: Option<String>,
    pub transform: TransformData,
    pub shape: CollisionShape,
    pub blocks_movement: bool,
}
```

Il runtime deve consumare `manifest.blockers`, oppure l'exporter deve convertirli in `props` bloccanti. Non lasciare due sistemi paralleli senza una regola chiara.

**Decisione consigliata**: usare `blockers` per i volumi gameplay e riservare `props` agli oggetti di gioco collocabili.

---

## 8. Runtime movement design

### Query di superficie

Implementare una query precisa:

```rust
pub fn ground_at(
    &self,
    x: f32,
    z: f32,
    preferred_surface: Option<SurfaceId>,
    reference_y: f32,
) -> Option<GroundContact>
```

Regole:

1. usare i bounds solo per filtrare rapidamente;
2. verificare la presenza reale nel triangolo/patch walkable;
3. calcolare l'altezza interpolando il triangolo;
4. calcolare la normale del triangolo;
5. rifiutare superfici oltre la pendenza massima;
6. tra superfici valide scegliere quella coerente con layer/current surface/reference Y;
7. non scegliere semplicemente la superficie più alta dell'intera mappa.

### Stato della superficie corrente

Dopo la fase mesh, aggiungere un riferimento alla superficie corrente del player, ad esempio:

```rust
pub struct CurrentSurface {
    pub id: String,
}
```

La superficie corrente serve per evitare che piattaforme sovrapposte o bounds adiacenti causino cambi casuali. Il cambio di superficie avviene solo quando:

- il nuovo triangolo è adiacente/raggiungibile;
- la variazione di altezza è compatibile con `max_step_height`;
- il nuovo layer è collegato tramite mesh/rampa;
- non c'è un blocker tra posizione corrente e candidato.

### Collisioni

`CollisionGrid` deve:

- leggere i blocker del manifest;
- rispettare la forma e la trasformazione world-space;
- controllare almeno X/Z per il movimento sul terreno;
- in seguito considerare Y quando sarà necessario distinguere piani sovrapposti;
- non usare blocker visuali non esportati come gameplay.

### Server/client

La funzione pura di stepping deve essere shared:

```text
server authoritative step
client prediction step
        ↓
shared SurfaceQuery + CollisionGrid + movement helper
```

Il server ricalcola sempre la superficie. La posizione Y inviata dal client non è autorevole.

---

## 9. Click-to-move e target

Il click deve risolvere il punto contro le superfici gameplay, non contro il piano `Y=0` e non contro il rettangolo completo della montagna.

Pipeline:

```text
camera ray
  -> candidate X/Z
  -> SurfaceQuery mesh
  -> triangolo walkable reale
  -> GroundContact { y, normal, surface_id }
  -> MoveCommand target X/Z + optional surface id
```

Il client può inviare `surface_id` come hint. Il server deve verificarlo e ricalcolare la superficie.

Se il click cade sulla roccia non walkable:

- non creare un target valido;
- non far camminare il player fino al bordo della piattaforma;
- mostrare eventualmente un indicatore rosso o nessun indicatore.

---

## 10. Piano di implementazione a slice verticali

Ogni slice deve iniziare con test RED, implementazione minima GREEN, test di regressione, clippy e revisione. Non introdurre codice speculativo prima del test che dimostra il comportamento.

### Slice 0 — Baseline e decisione del formato

**Valore**: avere un contratto stabile prima di ricreare la mappa Blender.

**Produzione coinvolta**: `manifest.rs`, loader, fixture JSON, documentazione del formato.

**Lavoro**:

- fotografare lo stato attuale dei cambiamenti sperimentali del movimento;
- decidere se il nuovo formato usa mesh inline nel JSON o un sidecar binario compatto;
- definire l'unità `1 Blender unit = 1 game unit = 1 metro`;
- definire gli id stabili e la distinzione `VISUAL/GAMEPLAY/COLLISION`;
- documentare un esempio minimo.

**Acceptance criteria**:

- un documento definisce il contratto Blender;
- un fixture minimo contiene ground, top platform, curved ramp e blocker;
- il fixture è caricato e validato dal loader;
- una superficie walkable non è rappresentata solo da bounds.

---

### Slice 1 — Mappa Blender minima con piattaforma e rampa dritta

**Attore**: level designer.

**Trigger**: esporta una mappa con ground, piattaforma alta e rampa.

**Risultato osservabile**: il player sale sulla rampa e non può salire sulla piattaforma dal lato sbagliato.

**Lavoro**:

- creare la nuova mappa Blender con le Collection definite sopra;
- creare `WALKABLE_Ground`;
- creare `WALKABLE_TestTop`;
- creare `RAMP_Ground_to_TestTop`;
- creare almeno due blocker laterali;
- esportare `.glb` + `.world.json`;
- implementare parsing delle superfici e dei blocker;
- implementare query triangolo -> altezza.

**Test RED**:

- query nel triangolo della rampa restituisce Y interpolata;
- query fuori dalla ribbon restituisce `None`;
- query sulla piattaforma restituisce la quota costante;
- blocker presente nel manifest impedisce il passaggio.

**Done when**:

- ground -> rampa -> top funziona server-authoritative;
- click su roccia/area vuota non crea movimento valido;
- client prediction e server usano lo stesso risultato.

---

### Slice 2 — Rampa curva/mezzaluna

**Attore**: level designer.

**Trigger**: esporta una ribbon mesh curva con una salita a mezzaluna.

**Risultato osservabile**: il player segue la mezzaluna, senza attraversare il rettangolo vuoto attorno alla curva.

**Lavoro**:

- usare una mesh gameplay curva low-poly;
- validare triangolazione e winding;
- supportare più triangoli con pendenze diverse;
- assicurare che bounds sia solo broad phase;
- aggiungere debug rendering dei triangoli walkable.

**Test RED**:

- punti dentro la mezzaluna risolvono la superficie;
- punti fuori dalla striscia ma dentro il bounds restituiscono `None`;
- due triangoli adiacenti producono una transizione continua;
- la query non sceglie una superficie visuale non walkable.

**Done when**:

- la curva è realmente attraversabile;
- il player non può tagliare attraverso il centro della mezzaluna;
- il player non può attraversare i lati rocciosi.

---

### Slice 3 — Superfici sovrapposte e livelli discreti

**Attore**: player.

**Trigger**: clicca una piattaforma alta mentre si trova sul terreno basso.

**Risultato osservabile**: il player non teletrasporta sulla piattaforma; può arrivarci solo attraverso una superficie di collegamento.

**Lavoro**:

- introdurre `CurrentSurface` o equivalente;
- definire la selezione per superficie corrente + adiacenza + quota;
- distinguere ground, top, bridge e upper floors;
- correggere spawn/respawn con ground snap verificato;
- validare cambi di layer soltanto attraverso una rampa.

**Test RED**:

- target alto senza rampa non è direttamente raggiungibile;
- target alto con rampa è raggiungibile;
- due superfici sovrapposte non producono switching casuale;
- spawn viene risolto sulla superficie corretta.

**Done when**:

- ground, mountain 1, mountain 2 e piani del castello possono coesistere;
- ogni quota è raggiungibile solo dal percorso corretto.

---

### Slice 4 — Pendenze e superfici non walkable

**Attore**: level designer.

**Trigger**: esporta rampa troppo ripida o roccia quasi verticale.

**Risultato osservabile**: la roccia non viene scelta come superficie valida e la rampa oltre il limite viene rifiutata.

**Lavoro**:

- calcolare normale e angolo per triangolo;
- applicare `max_walkable_slope_deg` globale/per superficie;
- validare triangoli degeneri, invertiti e verticali;
- generare errori dell'exporter con object id e triangle index.

**Test RED**:

- triangolo con pendenza 30° è walkable se il limite è 45°;
- triangolo con pendenza 60° non è walkable;
- normale invertita viene segnalata;
- mesh senza triangoli validi rifiutata.

---

### Slice 5 — Blocker Blender realmente attivi

**Attore**: level designer.

**Trigger**: sposta un blocker in Blender e riesporta.

**Risultato osservabile**: il bordo del cliff cambia nel gioco senza modifiche manuali al codice.

**Lavoro**:

- definire `BlockerData` spaziale;
- leggere transform e CollisionShape dall'exporter;
- includere blocker in `CollisionGrid::build`;
- rimuovere la dipendenza implicita da `props` per i blocker di mappa;
- aggiungere log di conteggio e ids caricati.

**Test RED**:

- il manifest contiene transform/shape del blocker;
- CollisionGrid conta i blocker;
- un blocker fuori dal percorso non blocca;
- un blocker sul bordo blocca;
- il corridoio della rampa resta aperto.

---

### Slice 6 — Mappa di validazione completa

**Attore**: level designer e QA.

**Trigger**: avvia la nuova mappa demo.

**Risultato osservabile**: una mappa contiene terreno, due montagne, rampa a mezzaluna, castello a quattro piani e percorsi non validi bloccati.

**Scenario minimo**:

```text
Ground
 ├── Mountain 1 via rampa curva
 ├── Mountain 2 via switchback
 └── Castle Floor 1 -> 2 -> 3 -> 4 via rampe/scali
```

**Acceptance criteria**:

- il player cammina sul terreno basso;
- sale sulla montagna solo dalla salita autorizzata;
- non sale dalla parete rocciosa;
- non taglia la curva della mezzaluna;
- può attraversare tutti i piani del castello tramite i collegamenti;
- non cade né teletrasporta verticalmente;
- server e client non divergono durante prediction;
- i click su superfici non walkable sono rifiutati;
- la mappa viene caricata in headless server senza asset visuali.

---

## 11. Validazione Blender prima dell'export

L'exporter deve fallire con messaggi chiari se trova:

- id duplicati;
- oggetti gameplay fuori dalle map bounds;
- mesh con triangoli degeneri;
- normali invertite;
- pendenze superiori al limite;
- superfici con buchi non intenzionali;
- superfici walkable sovrapposte senza layer/priority;
- rampa senza collegamento dichiarato quando richiesto;
- blocker che copre completamente una rampa;
- blocker senza shape o transform;
- scala non applicata;
- oggetti con Custom Properties mancanti;
- mesh visuale erroneamente marcata walkable;
- altezza della piattaforma non coerente con il modello visuale oltre una tolleranza.

Output di validazione consigliato:

```text
Map validation: FAILED
- RAMP_Mountain_01_Crescent: triangle 42 exceeds max slope (57.2° > 45°)
- BLOCKER_Mountain_01_BackCliff overlaps 96% of ramp entrance
- WALKABLE_Mountain_01_Top: duplicate surface id
```

---

## 12. Debug tools indispensabili

Aggiungere una modalità debug attivabile solo in sviluppo:

- triangoli walkable colorati per superficie/layer;
- normali visualizzate come linee;
- bounds broad phase trasparenti;
- blocker wireframe;
- id della superficie corrente sopra il player;
- punto di query del click;
- altezza risolta e normale;
- motivo del rifiuto movimento:
  - `NoWalkableTriangle`;
  - `TooSteep`;
  - `StepTooHigh`;
  - `Blocked`;
  - `TargetSurfaceUnavailable`.

Senza questi overlay, un problema Blender e un problema runtime possono apparire identici.

---

## 13. Cose da non fare

- Non rendere tutta la montagna walkable solo perché è dentro un rettangolo.
- Non usare il nome `RAMP_*` come unica logica runtime.
- Non esportare solo `start/end` per rampe curve.
- Non mettere blocker importanti solo dentro un campo che il runtime non legge.
- Non usare la mesh visuale dettagliata come collisione server.
- Non scegliere sempre la superficie con altezza massima.
- Non permettere al client di decidere la Y autorevole.
- Non duplicare il modello visuale completo per ogni patch walkable.
- Non mescolare pareti, underside e pavimenti nella stessa mesh gameplay.
- Non correggere la geometria con grandi blocker globali: lasciare libero il percorso corretto e bloccare solo i bordi.

---

## 14. Ordine operativo consigliato per rifare la mappa

1. Duplicare il file Blender e creare una nuova scena di test.
2. Applicare scale e rotazioni agli oggetti gameplay.
3. Creare solo `VISUAL`, `GAMEPLAY` e `COLLISION`.
4. Costruire una piccola area ground + piattaforma + rampa dritta.
5. Non iniziare dalla mappa completa.
6. Esportare il fixture minimo e validarlo.
7. Aggiungere una rampa curva/mezzaluna.
8. Verificare che il rettangolo intorno alla curva non sia walkable.
9. Aggiungere una seconda quota/montagna.
10. Aggiungere blocker laterali lasciando libero il percorso.
11. Aggiungere il castello a piani multipli.
12. Solo alla fine aggiungere rocce, dettagli e decorazioni visuali.

La nuova mappa Blender deve essere considerata valida solo quando il fixture minimo funziona: **una superficie reale, una rampa reale, un bordo bloccato e un click fuori dalla superficie che viene rifiutato**.

---

## 15. Criteri finali di completamento

- [ ] Le superfici gameplay sono separate dai modelli visuali.
- [ ] Le rampe curve sono mesh walkable reali, non rettangoli con nome `RAMP`.
- [ ] Le colline walkable contengono solo le zone percorribili.
- [ ] Le piattaforme hanno quote discrete e ids stabili.
- [ ] I blocker hanno trasformazione e shape nel manifest.
- [ ] `manifest.blockers` è consumato dal runtime.
- [ ] `SurfaceQuery` interroga la mesh reale o una maschera equivalente.
- [ ] `ground_at` non usa la superficie più alta indiscriminatamente.
- [ ] `max_step_height` limita la salita per tick.
- [ ] `max_walkable_slope_deg` limita la pendenza.
- [ ] Spawn e respawn risolvono la superficie corretta.
- [ ] Click su area vuota/roccia non produce un target walkable.
- [ ] Server e client condividono la stessa query.
- [ ] Esistono test unitari per triangoli, rampe curve, overlap, pendenze e blocker.
- [ ] Esiste una fixture Blender/JSON riproducibile.
- [ ] Esiste una modalità debug per visualizzare mesh e blocker.
- [ ] `cargo test` passa.
- [ ] `cargo clippy -- -D warnings` passa.
- [ ] Il test manuale completo di movimento è stato eseguito sulla nuova mappa.

---

## Domande aperte da decidere prima del codice

1. Il gameplay mesh viene serializzato direttamente nel `.world.json` oppure in un asset sidecar binario?
2. Le scale visuali saranno trattate come rampe lisce nel gameplay o come gradini reali?
3. Le discese oltre `max_step_height` sono sempre permesse oppure serve una regola di caduta?
4. Il player può attraversare liberamente tra superfici sovrapposte alla stessa quota?
5. Il layer/surface id deve essere replicato al client o mantenuto solo come stato locale/server?
6. I blocker saranno sempre box, oppure serviranno capsule/mesh polygonali per rocce irregolari?

**Decisione raccomandata per la prima versione**: mesh gameplay triangolata inline nel manifest, blocker box, scale trattate come rampe lisce, superficie corrente mantenuta nel runtime, debug overlay obbligatorio.