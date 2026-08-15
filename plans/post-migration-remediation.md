# Bonifica post-migrazione a SpacetimeDB

Audit condotto su quattro dimensioni indipendenti — codice morto, duplicazione,
uso del database, divario di funzionalità — con verifica incrociata delle
affermazioni contro il codice e contro la documentazione ufficiale 2.8.1.

Le quattro analisi convergono sulla stessa diagnosi: **il modulo server è la
parte riuscita, il client è dove la migrazione si è fermata.**

Il porting lato server è sostanzialmente completo, e in sette punti è *migliore*
dell'originale. Quello che manca sta quasi tutto nel client, rimasto attaccato a
lightyear: `ClientTransportPlugins` non viene più registrato, quindi ogni query
che filtra su `ConnectedClient`, `Predicted` o `MessageSender` non trova mai
nulla — **in silenzio, senza un errore di compilazione**.

## Il verdetto in cinque numeri

| | |
|---:|---|
| **0** | entità disegnate a schermo |
| **5 / 15** | azioni del giocatore che arrivano al server |
| **6 / 23** | tabelle sottoscritte dal client |
| **~10.000** | righe di codice morto o legacy |
| **~95%** | del gameplay portato nel modulo |

---

## 1. I cinque blocchi

In ordine: finché il primo non è risolto, gli altri non si vedono nemmeno.

### 1.1 Il gioco non disegna niente — BLOCCANTE

`spawn_entity_meshes` richiede `&EntityColor` come componente **obbligatorio**.
Lo inseriva lightyear via `Replicate`; il bridge SpacetimeDB non lo inserisce
mai. L'unica costruzione di `EntityColor` rimasta in tutto il codice vivo è
dentro un `#[cfg(test)]`.

Nessuna entità riceve `Mesh3d` o `Transform`. A cascata muoiono anche
`sync_transforms`, `anchor_player_model` e `update_colors`. Personaggi, nemici e
boss esistono nella ECS e sono invisibili.

- `crates/presentation/src/renderer.rs:166` — la query
- `crates/client/src/stdb/plugin.rs:339-374` — `apply_entity`, che non lo inserisce
- `crates/presentation/src/scenes/base/systems.rs:353` — dentro `mod tests`, aperto a riga 193

**Fix**: inserire `EntityColor` in `apply_entity` (colore derivato da `entity_id`
o nuova colonna `game_entity.color`), oppure renderla `Option` nel renderer.

### 1.2 Dieci azioni su quindici non arrivano al server — BLOCCANTE

La UI invia ancora tutto via `MessageSender<T>` con `With<ConnectedClient>`. Quei
sistemi girano ogni frame, eseguono la logica di targeting, cooldown e facing, e
poi iterano su una query sempre vuota.

Il paradosso: `crates/client/src/stdb/commands.rs` contiene già i wrapper
tipizzati verso i reducer — e **otto delle dieci funzioni non hanno un solo
chiamante**. Il percorso nuovo è scritto, compila, e nessuno lo usa.

| Azione | Invio morto | Reducer pronto |
|---|---|---|
| Lanciare una spell (Q/W/E) | `spells/input.rs:44` | `cast_spell` |
| Rilasciare un canalizzato | `spells/input.rs:45` | `release_cast` |
| Cliccare la hotbar | `spells/ui.rs:302` | `cast_spell` |
| Gesto Eidolon | `spells/eidolon_input.rs:58` | `eidolon_cast` |
| Equipaggiare / togliere | `ui/inventory/systems.rs:381-382` | `equip_item` / `unequip_item` |
| Drag & drop inventario | `ui/inventory/drag.rs:218-220` | `move_item` |
| Scrivere un'iscrizione | `ui/inscription/systems.rs:433` | `set_inscription` |
| Scegliere l'abilità dell'arma | `ui/inscription/systems.rs:435` | `set_ability_selection` |
| Fermarsi | nessuna UI | `stop` — **manca anche il wrapper** |

Funzionano solo `join`, `heartbeat`, `move_to`, `respawn` e `set_hotbar_spell`.

### 1.3 Il client non riceve quasi niente di quello che il server produce — BLOCCANTE

La subscription copre sei tabelle e **zero tabelle evento**. Il modulo inserisce
diligentemente `damage_event`, `spell_visual_effect`, `cast_ended` e
`player_message`; nessuno le sottoscrive, quindi nessun `on_insert` scatta. La
presentation legge ancora `MessageReader<SpellCastProgress>` di lightyear, un
canale che nessuno alimenta più.

Non è solo che la UI non invia: **non riceve**.

| Tabella non sottoscritta | UI che resta a secco |
|---|---|
| `cast_state` | Cast bar (463 righe) |
| `cooldown` | Overlay cooldown sulla hotbar |
| `projectile`, `aoe_region` | Proiettili e aree: non disegnati |
| `crowd_control` | CC bar (305 righe) |
| `stat_modifier`, `periodic_effect` | Buff e debuff |
| `boss_state`, `threat` | Boss bar (320 righe) |
| `known_glyphs` | Pannello iscrizioni (523 righe) |
| 4 tabelle evento | Effetti visivi, danno fluttuante, notifiche |

### 1.4 Si cammina sottoterra e attraverso i muri — BLOCCANTE

Il tick del server chiama `step_towards`, che è una retta pura. Il vecchio server
usava `step_on_terrain` con `snap_to_ground`, `max_step_height`, scivolamento
sugli ostacoli e collision test a raggio 0.45.

Il beffardo: **i dati ci sono già**. `world.rs` costruisce `SurfaceQuery` e
`CollisionGrid` a ogni avvio (circa 1 MB residente) ed espone `ground_at`,
`ground_height`, `is_blocked`. Nessuna di queste tre funzioni ha un chiamante
fuori da `world.rs`. Il server paga la griglia di collisione e poi la ignora.

Peggiorato dal client, che risolve il click sul piano Y=0 e manda `y: 0.0`
letterale. Su `map_02` l'origine sta sotto circa 4,9 m di collina: il personaggio
nasce sul terreno e al primo click ci si infila sotto.

- `crates/stdb-module/src/sim/movement.rs:31`
- `crates/shared/src/movement.rs:325-465` — lo stepper da spostare in `domain`
- `crates/client/src/stdb/plugin.rs:539-543`

### 1.5 Il personaggio nasce senza niente — BLOCCANTE

`join` inserisce inventario, equipaggiamento e glifi *vuoti*. Il vecchio server
dava un kit di 10 oggetti e un vocabolario di 3 essenze + 3 modificatori.
`grant_item` esiste nel modulo e ha zero chiamanti.

Conseguenza a catena: anche ricablando tutta la UI del punto 1.2, equipaggiare e
incidere resterebbero **irraggiungibili**, perché non c'è nulla da equipaggiare e
nessun glifo da incidere.

- `crates/stdb-module/src/reducers/lifecycle.rs:187-206`
- `crates/stdb-module/src/reducers/items.rs:542` — `grant_item`
- vecchio: `crates/server/src/persistence/repository/player.rs:567-621`

---

## 2. Regressioni introdotte dalla migrazione

Cose che prima funzionavano e adesso sono rotte o peggiorate — distinte dai
blocchi sopra, che sono cose non ancora collegate.

### 2.1 Il boss respawna arrabbiato

`kill()` assegna 30 secondi di respawn a ogni non-giocatore, boss compreso. Ma
`tick_respawns` non tocca mai `boss_state`: il drago torna a vita piena con
`phase = Enraged`, `is_engaged = true` e la threat table ancora sporca. Prima il
boss non respawnava affatto.

`crates/stdb-module/src/sim/combat.rs:729-733` e `:172-196`

### 2.2 Healing Circle non cura nessuno

Il payload è `ApplyModifier` con targeting `CasterOnly`: `aoe_region` non ha
colonne per nessuno dei due, quindi la regione non è persistibile e viene
risolta all'istante del cast su chi è dentro il cerchio. Ma il cerchio si centra
fino a 12 unità di distanza con raggio 4 — **il lanciatore non è mai dentro**.
Prima la regione viveva 3 secondi e ci si poteva camminare dentro.

Stessa causa, altri sintomi: `stun_field` perde i suoi 0,5 s di telegrafo,
`tail_sweep` e `wing_buffet` diventano istantanei, e le AoE a cono persistenti
non vengono mai scritte perché manca la colonna dell'apertura.

`crates/stdb-module/src/sim/spells.rs:469-486`

### 2.3 Mutazione durante l'iterazione, nel punto più caldo

`sim/movement.rs:16` itera `game_entity` e la aggiorna dentro il ciclo. Due altri
file dello stesso modulo documentano esplicitamente che non si può fare —
`sim/combat.rs:210` («mutating a table while iterating it is undefined here») e
`sim/crowd_control.rs:45` — e infatti raccolgono prima in un `Vec`. Il movimento
è l'unico posto che viola la regola, ed è quello che gira su ogni entità a ogni
tick.

### 2.4 Perdite puntuali

- **DoT/HoT si accumulano** — `periodic_effect` fa `insert` incondizionato invece
  del refresh che `stat_modifier` fa correttamente.
- **Niente attribuzione** — `stat_modifier` ha perso `source` e `kind`
  (buff/debuff), `damage_event` ha perso `source`, `crowd_control` ha perso
  `total_seconds` (la CC bar non può calcolare la frazione) e `source`.
- **Aggro range globale** — costante 10.0 per tutti; il goblin ne aveva 8.0.
- **Il movimento non è bloccato durante i cast**, e l'interruzione misura da
  `start_position` invece che dal tick precedente: combinato con il client che
  rimanda `move_to` ogni 100 ms mentre il tasto è premuto, **i cast a tempo si
  autocancellano**.
- **I personaggi offline restano nel mondo** — il vecchio server faceva `despawn`
  al disconnect; ora l'entità resta viva, visibile, aggredibile e uccidibile.
- **`ENEMY_RESPAWN_SECONDS` è 30.0 nel modulo e 10.0 nel dominio**, con un
  commento che sostiene siano uguali.

---

## 3. Codice morto e legacy

Circa 10.000 righe. La categoria peggiore non è quella che non compila — è quella
che compila, è registrata, e sembra viva.

### 3.1 Sistemi registrati che non possono fare nulla

| Sistema | Cosa aspetta invano |
|---|---|
| `predict_move_to_target` | `Client`, `IsSynced`, `Predicted`, `Player`, `ActionState<Inputs>` — fallisce in quattro modi |
| `spawn_entity_meshes`, `update_colors` | `EntityColor` |
| `cast_spells_on_key`, `cast_eidolon_abilities_on_key` | `MessageSender` + `ConnectedClient` |
| `dispatch_visual_effects` | una coda che nessuno scrive — con tutti gli `animate` delle 11 sotto-mod incantesimo |
| `read_cast_progress`, `read_cast_ended` | idem |
| `update_boss_bar`, `update_boss_banner` | `Boss`, `BossArena`, `BossPhase` |
| `sync_screen_cc_bars` | `CrowdControlState` |
| l'intera UI Inscription | `KnownGlyphs` obbligatorio in query: la finestra non si apre mai |
| `draw_ability_aim_preview` | `KnownGlyphs` |

In più: **due sistemi rispondono al tasto destro**. `select_move_target` disegna
l'anello di feedback all'altezza del terreno e invia un comando morto;
`send_move_commands` invia davvero, ma verso `y=0`. L'anello e la destinazione
sono due punti diversi.

### 3.2 Da cancellare

| Cosa | righe | Nota |
|---|---:|---|
| `crates/server/` | 8.243 | Il vecchio server. Serve come riferimento finché il porting non è verificato |
| `crates/client/src/network/` | 322 | Mai istanziato — ma è la sorgente di `ConnectedClient`, che 12 query filtrano ancora |
| `presentation/src/player_movement.rs` | ~230 | Terza implementazione della predizione, morta |
| `shared/src/network/protocol.rs` | ~200 di 302 | Parzialmente vivo: i *tipi* servono, il transport no. `ProtocolPlugin` (126 righe) non è mai montato |
| `shared/src/entity/spawn.rs` + `definition.rs` | ~265 | `spawn_entity` non è chiamato da nessun crate vivo |
| `crates/client/src/input/` | 18 | Puro re-export che nessuno usa |
| `Dockerfile` | — | Costruisce il vecchio server con feature **che non esistono più** e `ENTRYPOINT ["./game", "server"]`, sottocomando rimosso |
| `.env.example`, servizi `postgres` e `server` nel compose | — | Interamente Postgres |

Più: `sea-orm`, `sea-orm-migration`, `tokio`, `uuid` dichiarate in
`[workspace.dependencies]` e usate da nessuno; `lightyear` dichiarata dal binario
che non la nomina più; `AppMode::Server` e `HostClient` mai costruite; il
pacchetto ancora chiamato `bevy_lightyear_game`; il filtro di log di default che
punta a `lightyear=info`.

---

## 4. Logica duplicata

| Regola | Le due versioni | Divergenza |
|---|---|---|
| **Passo di movimento** | `shared/movement.rs:325` `step_on_terrain` · `domain/movement.rs:41` `step_towards` | Terreno + collisioni vs retta. Soglie di arrivo **0.05 vs 0.001**, e il doc di `step_towards` sostiene falsamente di replicare il vecchio server |
| **Unità di velocità** | `MovementStats.speed` (per tick, 0.15) · `StdbAuthoritative.speed` (per secondo, 9.0) | Lo stesso concetto in due componenti del *medesimo client*, in due unità. La costante 60.0 è riscritta in tre posti |
| **Fold dei modificatori** | `shared/movement.rs:49` `effective_value` · `sim/combat.rs:645` `apply_modifiers` | Ordine del `Vec` vs due passate deterministiche. La UI mostra un numero che il server non calcola mai così |
| **Conversioni riga↔dominio** | `stdb-module/rows.rs` · `client/stdb/plugin.rs:399-453` | Otto conversioni riscritte a mano sul client. L'ordine dei 10 slot equipaggiamento esiste in **tre** copie letterali |
| **Registry di gioco** | `sim/spells.rs:89-121` · `reducers/items.rs:78-102` | Cinque `OnceLock` duplicati. E `sim/combat.rs:613` ricostruisce l'intero catalogo oggetti **a ogni ricalcolo di stat** |
| **Predicati di morte** | `is_dead` · `is_alive` · 9 inline | `is_alive` controlla anche la salute, `is_dead` no: un bersaglio a 0 HP non ancora marcato è saltato dal targeting ma resta curabile |
| **Respawn** | `sim/combat.rs:171` (mob) · `reducers/combat.rs:29` (player) | Il mob respawna ancora stunnato e con i debuff; il player respawna senza ricalcolo delle stat |
| **Direzione di sguardo** | quattro implementazioni | Soglie diverse, e `shared/movement.rs` usa l'offset 3D *incluso Y*: se il bersaglio è più in alto, il personaggio guarda in su |
| **Distanza orizzontale** | ~15 riscritture | Le canoniche esistono già in `domain` ma sono private |

**Dieci commenti mentono.** Il doc di `commands.rs` dice «It now calls a reducer»
— nessuno lo chiama. `sim/ai.rs:571` dice che `apply_damage` «dovrebbe» chiamare
`accrue_threat` — lo fa già. `sim/combat.rs:205` dice che gli effetti periodici
non sono portati — lo sono. Vale la pena riscriverli insieme al codice: sono il
motivo per cui alcune di queste duplicazioni sembrano intenzionali.

---

## 5. Uso di SpacetimeDB

### 5.1 Gli errori del server vengono buttati via

I binding generano `cast_spell_then(...)`, che restituisce il
`Result<(), String>` del reducer. Il codice usa solo la forma fire-and-forget.
Tutti gli `Err("nome occupato")`, `Err("inventario pieno")`,
`Err("fuori portata")` scritti con cura nel modulo non arrivano mai a schermo.
`join` passa a `Screen::InGame` in modo ottimistico: se il nome è occupato, il
giocatore resta in un mondo vuoto senza spiegazione.

### 5.2 Tutti possono leggere l'inventario di tutti

23 tabelle su 24 sono `public`, senza alcun filtro di visibilità. `inventory`,
`equipment`, `player_stats`, `known_glyphs` sono leggibili da qualsiasi client,
via subscription o via `POST /v1/database/bevymmo/sql`. `player_message` ha un
campo `target` — cioè è un canale privato — ed è broadcastato a tutti.

La documentazione sconsiglia le RLS (sperimentali, richiedono
`features = ["unstable"]`) e indica le **view** come strumento di controllo
accessi: filtrano per riga *e* proiettano colonne, e possono leggere tabelle
private.

### 5.3 Occasioni non colte

- **Subscription per prossimità.** Oggi ogni client scarica il mondo intero. Il
  query builder tipizzato è già generato nei binding. L'indice `cell`
  multi-colonna copre `cell_x = A AND cell_z BETWEEN …` ma non due range
  simultanei: serve una colonna `cell_key: i64` impacchettata e indicizzata. La
  subscription non si aggiorna da sola al movimento — la doc prescrive il pattern
  *subscribe before unsubscribing*.
- **Indici mancanti su `kind` e `state`.** Sono enum senza payload, quindi
  indicizzabili. Con quei due indici tre delle cinque scansioni complete per tick
  diventano scansioni di indice.
- **Il tick è una transazione monolitica** a 20 Hz: cinque scansioni complete più
  `expire_stale_presence`, che serve un timeout di 15 secondi e gira 300 volte
  più del necessario. Andrebbe spezzato in scheduled reducer con intervalli
  diversi — attenzione però all'ordine, che oggi è semanticamente necessario.
- **`with_confirmed_reads` non è impostato**, quindi vale il default `true`: il
  server aspetta la conferma di durabilità prima di inviare gli update. La doc
  dice testualmente di disattivarlo per i giochi real-time. È una riga.
- **Nessuna riconnessione.** `connect()` gira una volta in `Startup`; se
  fallisce, `StdbConnection` non viene mai inserita e ogni sistema resta spento
  per sempre. `on_disconnect` logga e basta. E `frame_tick` su connessione caduta
  logga un errore *per frame*.
- **`entity_stats` è un materializzato a mano** che va ricalcolato da sei punti
  diversi; una dimenticanza produce statistiche stantie. È il caso d'uso di una
  `view` — ma le view non possono usare `.iter()` né scrivere, quindi va prima
  separato `current_health`/`current_mana` in una tabella a parte.

---

## 6. Quello che invece è migliorato

Perché la bonifica non deve buttare via anche questo.

- **Autenticazione vera.** Prima il netcode girava con chiave privata a zero e il
  personaggio era identificato dal nome: chiunque ne conoscesse uno poteva
  assumerlo. Ora la chiave è `Identity`, emessa e verificata dal database.
- **Niente più perdita di sessione.** Prima l'unico salvataggio era uno snapshot
  al disconnect, e non era transazionale: un crash perdeva tutto. Ora scrivere
  una riga *è* salvarla, nella stessa transazione.
- **Portata dei cast validata lato server** — prima il client era creduto sulla
  parola.
- **Il credito del danno funziona**: prima Fireball non generava minaccia perché
  il proiettile non portava la sorgente.
- **Niente più cure ai cadaveri**, e i proiettili non ri-uccidono chi è già morto.
- **I nemici respawnano e tornano allo spawn point** quando perdono il bersaglio.
- **Colonne vere invece di JSON in TEXT**: l'inventario è interrogabile con
  `spacetime sql`.

---

## 7. Correzioni ad assunzioni precedenti

| Si era detto | In realtà |
|---|---|
| «Le spell non costano mana» — regressione | **Mai esistito.** `SpellConfig` non ha mai avuto un campo costo e il vecchio server non toccava il mana. Il modulo ha *aggiunto* `current_mana` e la rigenerazione. È debito di design, non di porting |
| «`ModifierOp::Override` non è rappresentabile» | **Metà vero.** Funziona per i bonus da oggetto; è scartato solo per i modificatori a tempo. Nessun contenuto attuale lo emette |
| «`cinder_storm` manca» — colpa della migrazione | **Pre-esistente**: cancellata il 7 agosto nel crate split. Aggravante nuova: consuma il `break` della rotazione, quindi con due giocatori in arena il drago salta il turno e logga a 19 Hz |
| «Manca il livello personaggio» | **Mai esistito.** `MinLevel` era un hook mai letto |
| «`apply_damage` e `apply_healing` divergono sul controllo di morte» | **Falso.** Usano lo stesso controllo. La divergenza vera è fra `is_dead` e `is_alive` |
| «Il mirroring funziona» dopo lo smoke test | **Incompleto.** Le righe diventavano entità ECS, ma invisibili: era stata verificata metà della catena e riportata l'intera |

---

## 8. Ordine di bonifica

Pensato perché ogni passo sia verificabile a schermo prima del successivo.

1. **Rendere visibile il mondo.** Inserire `EntityColor` nel bridge, o renderla
   opzionale nel renderer. È il passo che trasforma tutto il resto da teoria a
   qualcosa che si può guardare.

2. **Terreno, collisioni e click a quota giusta.** Spostare `step_on_terrain`,
   `try_step` e `snap_to_ground` in `bevymmo_domain`, chiamarli dal tick con i
   dati che `world.rs` già carica, e far usare al client `resolve_ray_to_ground`
   invece del piano Y=0. Nello stesso passo: raccogliere prima di aggiornare in
   `sim/movement.rs`.

3. **Kit iniziale e vocabolario.** Chiamare `grant_item` in `join` con i 10
   oggetti, e seminare le 3 essenze + 3 modificatori. Senza questo il ramo
   oggetti resta irraggiungibile anche dopo il ricablaggio.

4. **Ricablare le dieci azioni mute.** Sei file, da `MessageSender` a
   `bevymmo_client::stdb::commands`. I wrapper esistono già tutti tranne `stop`.
   Passare a `*_then()` così gli errori del server arrivano a schermo.

5. **Estendere le sottoscrizioni.** Le 17 tabelle mancanti più le 4 evento, e
   mappare le righe sui componenti che la presentation già interroga. Riporta in
   vita cast bar, CC bar, boss bar, buff, iscrizioni, danno fluttuante ed effetti
   visivi — tutta UI già scritta.

6. **Cancellare il morto.** `crates/client/src/network/`,
   `presentation/player_movement.rs`, il transport in `protocol.rs`,
   `entity/spawn.rs`, il Dockerfile, `.env.example`, i servizi Postgres nel
   compose, le dipendenze inutilizzate. Poi `crates/server`, quando i passi 1-5
   sono verificati.

7. **Collassare i duplicati.** Un solo stepper, una sola unità di velocità, un
   solo fold dei modificatori, un solo predicato di morte, un solo `resurrect`,
   un solo modulo registry. E riscrivere i dieci commenti che mentono.

8. **Riparare le regressioni.** Respawn del boss, `healing_circle` e i telegrafi
   AoE (richiede colonne su `aoe_region`), refresh dei periodici, blocco del
   movimento durante i cast, `despawn` dei personaggi offline.

9. **Sicurezza e prestazioni.** Tabelle private più view per-utente, indici su
   `kind` e `state`, `with_confirmed_reads(false)`, riconnessione, tick spezzato
   per intervalli, subscription per prossimità.

10. **Rete di sicurezza.** Zero test sul modulo oggi, contro 178 nel dominio. E
    nessuna CI: il workspace passa verde mentre il modulo è rotto — è già
    successo. Riscrivere `smoke-test-checklist.md`, che è la definizione scritta
    di «come funzionava prima», sulle modalità che esistono adesso.
