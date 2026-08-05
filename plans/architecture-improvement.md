# Piano: architettura gameplay, networking e presentazione

**Stato**: implementazione iniziale completata — bootstrap, ruoli, spawn comune, UI e documentazione aggiornati. Rimane opzionale lo spostamento fisico dello stato gameplay fuori da `network::protocol`.

## Obiettivo

Rendere il gioco Bevy + Lightyear facile da estendere con nuove entità, azioni, UI e scene, mantenendo il server autorevole e separando con chiarezza bootstrap, simulazione, replica e presentazione.

## Decisioni architetturali

### Da mantenere

- **Plugin per feature**, non un plugin per file: `EntityPlugin`, `UiPlugin`, `ScenesPlugin` sono aggregati validi; `PlayerPlugin`, `EnemyPlugin`, `EntityBarPlugin` e `ScoreboardPlugin` sono feature plugin validi.
- **Composizione ECS**: componenti e query invece di gerarchie OOP.
- **Server-authoritative + client prediction** per il movimento.
- **Observer Bevy** per reagire alla nascita di entità o al cambio di connessione.
- **`EntityDefinition`** come contratto statico per entità standard server-spawned (enemy/NPC/proiettili). Non trasformarlo in un framework di factory.

### Da introdurre, solo perché risolve problemi già presenti

1. **`AppMode` come stato esplicito** (`Client`, `Server`, `HostClient`), al posto di inferire il ruolo dalla presenza di `ClientConnectionConfig` o `ServerConnectionConfig`.
2. **Bootstrap centralizzato**, per evitare tre sequenze quasi identiche in `src/main.rs`.
3. **Spawn spec/bundle del player**, per eliminare la duplicazione fra `spawn_entity::<T>()` e lo spawn manuale in `network/server.rs`.
4. **Riferimenti UI diretti**, per evitare la navigazione `Children -> grandchildren` in ogni frame.
5. **Aggiornamento scoreboard a richiesta**, non despawn/respawn a ogni frame mentre `Tab` è premuto.
6. **Configurazioni nominate** per parametri gameplay e rete che oggi sono hard-coded.

### Da non introdurre ora

- Event bus generico per ogni aggiornamento UI.
- Factory/abstract factory runtime per le entità.
- Componenti duplicate `GameplayPosition`/`NetworkPosition`: `Position` è già lo stato di gameplay replicato e il renderer può leggerlo.
- Layer `domain`, `application`, `infrastructure` separati: per questa codebase ECS aggiungerebbero attraversamenti e boilerplate senza valore.

## Struttura target

```text
src/
├── app/
│   ├── mod.rs                 # GamePlugin: registra feature comuni
│   ├── mode.rs                # AppMode / AppModeConfig e run conditions
│   └── bootstrap.rs            # plugin groups per client, server, host
├── network/
│   ├── mod.rs                 # NetworkPlugin e config condivisa
│   ├── client.rs              # trasporto, lifecycle e observers client
│   ├── server.rs              # trasporto, lifecycle e spawn autoritativo
│   └── protocol.rs            # soli dati e registrazioni Lightyear
├── gameplay/
│   ├── mod.rs                 # GameplayPlugin
│   ├── components.rs           # Health, Stats, GameEntity, PlayerName
│   ├── movement.rs             # input, simulazione server, prediction client
│   └── entities/
│       ├── mod.rs
│       ├── common.rs           # EntityDefinition + spawn spec comuni
│       ├── player/
│       └── enemy/
├── presentation/
│   ├── mod.rs                 # ClientPresentationPlugin
│   ├── scene/
│   │   └── base/
│   ├── renderer/
│   └── ui/
│       ├── entity_bar/
│       └── scoreboard/
└── main.rs                    # CLI -> AppModeConfig -> bootstrap -> run
```

> Questa è una **direzione**, non il primo refactor. Si adotta gradualmente: nessuna cartella verrà rinominata finché un slice non produce un confine utile e testato. In particolare, `plugins/` può restare il root corrente fino al completamento degli slice 1–3.

## Convenzioni da adottare

| Ambito | Regola |
|---|---|
| Plugin | Un plugin registra una capability osservabile; i plugin aggregati registrano solo figli. |
| Componenti | Nel file `components.rs` della feature; marker con nomi semantici (`GameEntity`, non `EntityMarker`). |
| Sistemi | `systems.rs` quando la feature ha più sistemi; nomi verbo + soggetto (`update_entity_bar_positions`). |
| Stato rete | Solo `network/protocol.rs` decide cosa è replicato; il gameplay non crea socket/transport. |
| Ruoli | `AppMode` decide client/server/host; non usare config transport come flag di ruolo. |
| UI | Ogni widget conserva i riferimenti alle sue parti, non scansiona i `Children` a ogni frame. |
| Scene/render | Solo client presentation; il server non registra window, renderer, scene o UI. |
| Config | Costanti di tuning in resource/config dedicata, con nomi di dominio. |

## Slice 0 — rete di sicurezza per i refactor

**Valore**: ogni successivo refactor mantiene comportamenti osservabili.

**Path**: server headless -> client connette -> player server-spawned -> replica -> renderer/UI.

**Acceptance criteria**:

- [ ] Test o harness ripetibile avvia un `App` server e verifica che il server registri la socket senza plugin di rendering.
- [ ] Test di protocollo registra tutti i componenti e messaggi previsti.
- [ ] Test ECS verifica che uno spawn enemy includa stato condiviso e `Replicate`.
- [ ] Checklist manuale documenta avvio `server`, due `client`, movimento e scoreboard.

**RED**: test per il bundle base e per la configurazione headless prima di refactorarlo.

**GREEN**: estrarre solo helper/test fixture minimi per costruire app senza window/GPU.

**Verifica**: `cargo test`, `cargo check`, smoke test manuale a tre processi.

## Slice 1 — ruoli applicativi espliciti e bootstrap unico

**Valore**: client, server e host-client hanno configurazioni prevedibili; aggiungere un plugin non richiede modifiche incoerenti in tre funzioni.

**Path**: CLI -> `AppModeConfig` -> plugin comuni -> plugin del ruolo -> `App::run`.

**Acceptance criteria**:

- [ ] Esiste un tipo `AppMode` che rappresenta `Client`, `Server`, `HostClient`.
- [ ] Una sola funzione decide quali plugin comuni, server-only e client-presentation registrare.
- [ ] Il comando `server` avvia in headless e non registra window/UI/renderer.
- [ ] Il comando `client` registra input, scene, renderer e UI.
- [ ] La modalità `host-client` dichiara e testa esplicitamente quali sistemi server e client devono convivere.

**RED**: test sulla matrice di registrazione plugin per ciascun `AppMode`.

**GREEN**: introdurre `app/mode.rs` e un builder piccolo; mantenere gli attuali moduli di feature e indirizzi/CLI.

**Pattern**: *Facade* leggera per il bootstrap; *Strategy tramite enum* per i ruoli. Non usare trait object.

**Verifica**: test di matrice + server headless + due client connessi.

## Slice 2 — sistemi gameplay eseguiti nel ruolo corretto

**Valore**: AI e simulazione autoritativa non dipendono accidentalmente da risorse transport e non si duplicano in host-client.

**Path**: `AppMode` -> run condition semantica -> FixedUpdate gameplay.

**Acceptance criteria**:

- [ ] `enemy_chase` e il movimento autoritativo girano solo nei ruoli che includono il server.
- [ ] Input, click indicator e prediction girano solo nei ruoli che includono il client.
- [ ] I run condition non leggono `ServerConnectionConfig`/`ClientConnectionConfig` come marker di ruolo.
- [ ] Un test di schedule o unit test delle condizioni copre client, server e host-client.

**RED**: test delle run condition per tutti e tre i ruoli.

**GREEN**: sostituire gradualmente `resource_exists::<...>` con condizioni `AppMode` nominate.

**Pattern**: *State/Strategy* tramite enum e run conditions Bevy.

**Verifica**: test + smoke test host-client senza doppio movimento/AI.

## Slice 3 — spawn delle entità con confini chiari

**Valore**: aggiungere una nuova entità server-spawned è meccanico; il player mantiene in un solo punto le proprie esigenze di ownership/prediction.

**Path**: evento `Connected` -> player spawn spec -> componenti gameplay -> componenti Lightyear ownership/replica.

**Acceptance criteria**:

- [ ] Le componenti comuni (`Health`, `Stats`, `EntityState`, posizione, colore) vengono composte da un unico bundle/spec riusabile.
- [ ] `EntityDefinition` resta per le entità standard; player usa uno spec che accetta il suo owner/peer ID, senza copiare i default a mano.
- [ ] `EntityMarker` viene rinominato in `GameEntity` e tutte le query/commenti sono aggiornati.
- [ ] L’aggiunta di un enemy/NPC non richiede cambiare il bootstrap o il renderer.

**RED**: test che confronta componenti base di player ed enemy e test dello spawn player con owner.

**GREEN**: estrarre `GameEntityBundle` o `GameEntitySpawn` (dato statico); evitare macro e generics non necessari.

**Pattern**: *Builder data-oriented* / *Factory Method statico* già espresso da `EntityDefinition`.

**Verifica**: test bundle + server con due client e un enemy replicati.

## Slice 4 — UI flottante robusta e leggibile

**Valore**: ogni replica visibile con `Position` e `Health` mostra una sola barra, che segue l’entità e aggiorna correttamente nome/HP senza dipendere dall’ordine di replica.

**Path**: replica componenti -> attach `EntityBar` -> aggiornamento posizione -> aggiornamento contenuto -> cleanup target rimosso.

**Acceptance criteria**:

- [ ] Una barra contiene riferimenti diretti a `name_text`, `hp_fill` e `hp_text`, invece di percorrere `Children` e grandchildren.
- [ ] Il sistema di posizionamento e quello di contenuto sono separati e ciascuno ha query mirate.
- [ ] Il widget è creato una sola volta per target dopo che esistono `Position` e `Health`.
- [ ] La rimozione del target rimuove il widget e non lascia marker/UI orfani.
- [ ] Con due client e un enemy, ogni client vede tre barre coerenti.

**RED**: test ECS per attach idempotente, cleanup e calcolo HP percentuale; test del mapping world-to-viewport dove possibile.

**GREEN**: aggiungere `components.rs` alla feature `entity_bar` e un componente composito che conserva gli ID figli.

**Pattern**: *Composite* (gerarchia UI Bevy) con una facade componente del widget. Nessun event bus.

**Verifica**: `cargo test` + smoke test visivo con due finestre.

## Slice 5 — scoreboard reattivo e tema UI minimo

**Valore**: tenere Tab aggiorna lo scoreboard soltanto quando cambia la lista giocatori; stile UI consistente senza valori copiati.

**Path**: `PlayerName` aggiunto/rimosso/cambiato -> scoreboard dirty -> rebuild/update lista -> Tab mostra/nasconde.

**Acceptance criteria**:

- [ ] Premere Tab non despawna e respawna tutti i figli in ogni frame.
- [ ] Join, leave e rename aggiornano la lista visibile alla successiva apertura o mentre è aperta.
- [ ] Colori, dimensioni font e spaziature comuni sono in una `UiTheme` piccola; non creare un design system generico.
- [ ] Titolo e righe hanno componenti semantici (`ScoreboardTitle`, `ScoreboardRow`).

**RED**: test ECS per nessun despawn quando non ci sono modifiche e per refresh dopo add/remove `PlayerName`.

**GREEN**: aggiornamento dirty/on-change; iniziare con rebuild soltanto quando changed, non con diff complesso per riga.

**Pattern**: *Observer/Change detection* nativi Bevy; *Resource* per la configurazione UI.

**Verifica**: test ECS + profiler/manual check: holding Tab non crea entità continuamente.

## Slice 6 — separare simulation state da rendering policy

**Valore**: il renderer può crescere (mesh differenti, materiali, asset) senza rendere `network/protocol.rs` un modulo di presentazione.

**Path**: stato gameplay replicato (`Position`, `EntityColor`, tipo entità) -> sistema client presentation -> `Mesh3d`, material e transform.

**Acceptance criteria**:

- [ ] `RendererPlugin` resta client-only e non contiene setup di scene.
- [ ] `BaseScenePlugin` è l’unico owner di camera, luce e terreno.
- [ ] Il renderer dipende soltanto da componenti di stato condiviso; evitare componenti “network” nominate nel renderer quando viene introdotto un modulo gameplay dedicato.
- [ ] L’allocazione di mesh/materiali viene centralizzata in cache/resource se il numero di entità cresce oltre il prototipo.

**RED**: test che il renderer non viene registrato sul server e test di mapping per almeno player/enemy.

**GREEN**: prima spostare fisicamente `Position`/`EntityColor` in un modulo `gameplay::components` mantenendo `ProtocolPlugin` come proprietario della loro registrazione; non duplicare i dati né aggiungere conversion systems.

**Pattern**: *Adapter* sottile della presentazione verso stato gameplay; caching resource solo quando misurato.

**Verifica**: server headless senza `Assets<Mesh>`; client renderizza player/enemy e UI.

## Slice 7 — pulizia e documentazione per contributor

**Valore**: un nuovo contributore sa dove aggiungere una feature senza leggere tutto il progetto.

**Acceptance criteria**:

- [ ] Rimuovere `network/shared.rs` se resta vuoto, oppure dargli una responsabilità reale e un nome coerente.
- [ ] Risolvere i warning di import inutilizzati senza modifiche comportamentali.
- [ ] Aggiornare `docs/create-a-new-plugin.md` con template feature plugin e checklist: componenti, sistemi, replica, presentation, test.
- [ ] Aggiungere un breve `docs/architecture.md` con diagramma dei confini e tabella ruolo -> plugin.

**Verifica**: `cargo fmt --check`, `cargo check`, `cargo test`, documentazione revisionata contro una feature nuova fittizia.

## Stato di avanzamento

- **Completato**: fondamenta di test (test `AppMode` e `GameEntityBundle`), bootstrap centralizzato, ruoli espliciti, run condition semantiche, `GameEntityBundle`, refactor EntityBar, scoreboard reattivo, tema UI, documentazione e rimozione del plugin vuoto.
- **Rinviato intenzionalmente**: spostamento fisico di `Position` e `EntityColor` da `network::protocol` a un modulo gameplay. Il renderer legge già soltanto stato replicato e il trasferimento non produce un beneficio comportamentale immediato; va eseguito quando sarà introdotto un vero modulo `gameplay/`.
- **Da aggiungere quando serve CI**: harness di integrazione che avvii server + due client senza GPU e smoke test grafico manuale/automated.

## Ordine consigliato

1. Slice 0 (test harness)
2. Slice 1 (bootstrap + ruoli)
3. Slice 2 (run conditions)
4. Slice 3 (spawn)
5. Slice 4 (entity bar)
6. Slice 5 (scoreboard)
7. Slice 6 (presentation boundary)
8. Slice 7 (cleanup/docs)

Ogni slice è una PR indipendente e chiude con `cargo fmt --check`, `cargo check`, test mirati e smoke test manuale. Prima di implementare ciascuno: definire e approvare i relativi test/criteri di accettazione.

## Questioni da decidere prima dello Slice 1

1. `host-client` deve essere una modalità supportata per sviluppo/debug o un target reale? Questa scelta definisce quanto investire nel suo comportamento e nei test.
2. Il server resterà sempre headless, oppure vuoi una modalità `server --visual` esplicita per debug? Il piano assume server headless per default.
3. I nomi player devono essere `Player <peer_id>`, scelti dal client, o derivati da un account? Il piano UI presume che `PlayerName` continui a essere autorevole sul server.
