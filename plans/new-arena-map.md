# map_02 → arena PvP simmetrica 100×100, ricca di strutture

## Concept
Tutta la mappa è l'arena, **simmetria rotazionale a 180°** (ogni cosa a Est ha il gemello invertito a Ovest) per i futuri spawn team blue (ovest) / team red (est). Boss rimosso. Scala 1:1, niente più `scale_map.py`.

## Layout (metri finali = unità Blender, bounds ±50)

```
 ██████████████████████████████████████████████   muro perimetrale h=7
 ██  [DAIS +0.4]                    [DAIS +0.4] ██   14×14 ai 4 angoli
 ██      muri copertura (N-O)   muri (N-E)     ██
 ██                                          ██
 ██   ○pilastro     ┌────────┐     pilastro○  ██   podio 18×18, y=+3
 ██                 │ PODIO  │                ██   anello largo ~40 m
 ██  TEAM BLUE      │+ring   │     TEAM RED   ██   rampe E/O: largh. 10,
 ██  spawn (−44,0)  └────────┘   spawn(44,0)  ██   percorrenza 10 (~17°)
 ██   [muro][muro]     rampe      [muro][muro]██   tasche di spawn protette
 ██                                          ██
 ██      muri copertura (S-O)   muri (S-E)   ██
 ██  [DAIS +0.4]                    [DAIS +0.4] ██
 ██████████████████████████████████████████████
```

### Strutture (tutte chiuse sotto la rotazione di 180°)
| Struttura | Quantità | Dettagli |
|---|---|---|
| Pavimento anello | 1 | 98×98 a y=0 |
| Podio centrale | 1 | 18×18, top +3.0, con anello decorativo incassato sul top (solo visuale) |
| Rampe podio E/O | 2 | larghezza 10, corsa 10; passaggio dietro: 30 m |
| Dais d'angolo | 4 | 14×14 a **+0.4** (sotto il max_step 0.45: ci si sale camminando, senza rampe né blocker) |
| Pilastri | 4 | 2.5×2.5×7 a (±14, ±30), visibili + collisione |
| Muri di copertura | 16 | 8 per lato, lunghezze 4–9 m, altezze 2.5–4 m, incluse **2 coppie a L** per lato, seed fisso + regole anti-ostruzione, tutti assialmente allineati (collisione AABB v1) |
| Muri tasca spawn | 4 | 2 per spawn (bassi, 2.5 m) a protezione della zona rigenerazione |
| Muri perimetrali | 4 | spessore 2, altezza 7 |
| Spawn | 2 | `player_spawn` a (±44, 0): il round-robin esistente alterna i giocatori est/ovest = preludio blue/red |
| NPC | 1 | `npc_greeter` come arbitro al centro del podio |

Blocker totali: 38 hand-authored (4 muri perimetrali + 6 lati podio Empty + 4 fianchi rampe Empty + 16 coperture + 4 pilastri + 4 tasche). I blocker "invisibili" restano **Empty** (le mesh verrebbero renderizzate dal client); i lati podio terminano a y=2.7 (0.3 sotto il top) per non bloccare chi cammina sul bordo.

## Cancellazioni
`WALKABLE_map_02` (terreno 360×360), 296 `Template_*`, 82 `Ramp_Edge_Visual_Rock_*`, `PLACEABLE_boss_dragon` (rimozione boss), purge materiali orfani. `__bevymmo_map_meta` aggiornato: bounds ±50, display_name "Arena", map_id resta `map_02`.

## Pipeline (zero modifiche al codice Rust)
1. Costruzione via Blender MCP con script parametrico (seed fisso per le coperture) + screenshot viewport da mostrarti + `save_mainfile`.
2. Export `.world.json`: `bevymmo_export_world.py` in-process, `build_manifest(resolution=32)` — superfici piatte, JSON ~200 KB (contro 4.2 MB).
3. Export GLB (Custom Properties ON, +Y Up, Apply Modifiers ON).
4. `generate_blockers_from_glb.py map_02` → atteso 0 AUTO; i 38 hand-authored preservati.
5. Verifica python: bounds ±50, 8 superfici, 38 blocker, 3 props, simmetria verificata (ogni oggetto est ha counterpart a (−x,−z)).

## Server, test, docs
6. `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings`.
7. `docker compose up -d spacetimedb` + `./scripts/stdb.sh reset` (obbligatorio: i personaggi esistenti hanno coordinate della vecchia mappa; reset = ripubblica a DB vuoto, `init` semina il mondo nuovo).
8. `cargo run -- client` per la verifica in-game.
9. Aggiornare la riga di map_02 in `docs/level-designer-guide.md` §14.

## Checklist gameplay
- Spawn alternati nelle due tasche dietro le rampe; anello percorribile a loop completo
- Salita rampe fluida (~17°); dais d'angolo salgono a piedi senza salti
- Podio, pilastri, coperture, muri perimetrali e tasche bloccano davvero; le due metà sono identiche ruotate di 180°
- Greeter sul podio; drago rimosso; nessuna via di fuga dall'arena

## Rischi coperti
- .blend committed e pulito: revert possibile in ogni momento
- Coperture con regole di spaziatura (muri ≤9 m contro corridoi ≥30 m): nessun percorso sigillato
- Export si ferma sui warning prima di toccare i file; verifiche programmatiche su simmetria e conteggi prima del publish
