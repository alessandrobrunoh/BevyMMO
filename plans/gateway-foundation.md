# Piano: base modulare del Gateway Axum

**Stato**: Attivo — piano da approvare prima dell'implementazione
**Area**: `apps/gateway`
**Branch proposta**: `feat/gateway-foundation`

## Obiettivo

Creare una base Axum piccola, modulare e verificabile per il sito Angular e per future API developer, mantenendo SpacetimeDB come autorità del gameplay e rimandando login/Postgres a una decisione separata.

## Risultato del primo milestone

Al termine del milestone il Gateway:

- è un membro del workspace Cargo;
- si avvia usando la configurazione condivisa del repository;
- espone endpoint pubblici e interni in namespace separati;
- espone `GET /health` e `GET /`;
- restituisce errori JSON stabili;
- ha test HTTP senza aprire necessariamente una porta TCP;
- ha un punto di estensione per l'adapter SpacetimeDB, senza introdurre query o logica di gioco prematura;
- resta deployabile senza Postgres e senza provider di autenticazione.

## Decisioni architetturali

### Decisioni per questo milestone

1. **Axum resta una facade HTTP**, non il server autorevole del gioco.
2. **SpacetimeDB resta owner** di personaggi, inventario, combattimento, mondo e stato realtime.
3. Le API pubbliche usano `/api/v1/...`.
4. Le API interne usano `/api/internal/v1/...`.
5. Health e liveness restano fuori dal versionamento API:
   - `GET /health`
   - `GET /health/live`
6. I DTO HTTP non sono le row generate da SpacetimeDB.
7. Nessuna password viene gestita dal Gateway in questa fase.
8. Nessun Postgres viene aggiunto finché non esiste un dato applicativo che ne richieda la persistenza.
9. Nessun endpoint pubblico esegue SQL arbitrario o invoca reducer arbitrari scelti dal client.

### Decisioni rimandate esplicitamente

- provider OIDC: SpacetimeAuth, Auth0, Clerk, Keycloak o altro;
- modalità di login del sito;
- API key e OAuth client credentials per sviluppatori;
- ruoli e scopes definitivi;
- dati applicativi che potrebbero richiedere Postgres;
- primo endpoint pubblico con dati reali di gioco;
- primo endpoint interno per amministrazione.

Queste decisioni non devono essere nascoste dentro lo skeleton.

## Fuori scope

Non implementare nel primo milestone:

- login email/password;
- registrazione utenti;
- refresh token o sessioni persistenti;
- Postgres, SeaORM o migrazioni;
- API key per sviluppatori;
- rate limiting distribuito;
- billing o store;
- sincronizzazione completa SpacetimeDB/Postgres;
- WebSocket proxy generico;
- API di combattimento o mutazioni gameplay;
- accesso SQL diretto dal frontend;
- CORS permissivo in produzione.

## Struttura target

La struttura deve crescere per capability, non per una separazione teorica di tutti i layer:

```text
apps/gateway/
├── Cargo.toml
└── src/
    ├── main.rs                 # bootstrap Tokio, config, listener, shutdown
    ├── app.rs                  # build_router e composizione dei router
    ├── config.rs               # parsing/validazione della config del gateway
    ├── state.rs                # AppState condiviso dagli handler
    ├── error.rs                # AppError -> risposta JSON stabile
    ├── auth/
    │   └── mod.rs              # placeholder minimo; niente login nel milestone
    ├── http/
    │   ├── mod.rs
    │   ├── health.rs           # health/live
    │   ├── public.rs           # /api/v1
    │   └── internal.rs         # /api/internal/v1
    ├── services/
    │   └── mod.rs              # confine per use case futuri
    └── adapters/
        ├── mod.rs
        └── spacetimedb.rs      # confine verso SpacetimeDB, inizialmente minimale
```

La struttura è un target graduale. Non creare moduli vuoti solo per riempire cartelle: ogni modulo deve essere introdotto quando ha un comportamento o un confine testato.

## Contratti HTTP iniziali

### `GET /health`

Risposta `200 OK`:

```json
{
  "status": "ok",
  "service": "bevymmo_gateway"
}
```

### `GET /health/live`

Risposta `200 OK` con lo stesso contratto di liveness. Non deve verificare SpacetimeDB o altri servizi: indica solo che il processo è vivo.

### `GET /`

Risposta `200 OK` con messaggio di benvenuto e nome servizio. Non deve rivelare segreti, token o configurazioni sensibili.

### `GET /api/v1`

Risposta `200 OK` con metadati minimi dell'API:

```json
{
  "name": "BevyMMO API",
  "version": "v1"
}
```

### `GET /api/internal/v1`

Nel primo slice può essere una risposta statica di discovery interna. Non deve diventare automaticamente un endpoint amministrativo pubblico.

La protezione vera delle API interne sarà uno slice successivo. Finché non esiste middleware di autenticazione, l'endpoint interno non deve contenere dati sensibili né operazioni distruttive.

## Acceptance Criteria globali

- [ ] `apps/gateway` resta un membro del workspace e `cargo check -p bevymmo_gateway` passa.
- [ ] `cargo test -p bevymmo_gateway` passa.
- [ ] `cargo clippy -p bevymmo_gateway --all-targets -- -D warnings` passa.
- [ ] Il Gateway carica `gateway.bind_addr` dalla configurazione condivisa.
- [ ] Il Gateway può essere avviato con `cargo run -p bevymmo_gateway`.
- [ ] `/`, `/health`, `/health/live` e `/api/v1` hanno risposte JSON documentate e testate.
- [ ] Public e internal router sono costruiti separatamente.
- [ ] Un errore di route sconosciuta restituisce un JSON coerente, non una risposta HTML casuale.
- [ ] Il codice HTTP non contiene logica di gameplay.
- [ ] Nessun Postgres o credenziale hardcoded viene aggiunto.
- [ ] La configurazione espone un punto chiaro per aggiungere in futuro issuer/audience OIDC senza fingere che l'autenticazione sia già implementata.

## Slices

Ogni slice segue: **RED → GREEN → MUTATE → KILL MUTANTS → REFACTOR**.
Prima di implementare ogni slice devono essere caricati i skill `tdd`, `testing`, `mutation-testing` e `refactoring`. I criteri della slice devono essere confermati prima di scrivere il codice.

### Slice 1 — Il Gateway espone una superficie HTTP modulare minima

**Valore**: il frontend e gli operatori hanno un processo HTTP avviabile con health-check, mentre il codice ha già confini pubblici e interni separati.

**Attore**: operatore locale o processo di orchestrazione.

**Trigger**: avvio di `cargo run -p bevymmo_gateway` e richiesta HTTP.

**Path**: `main.rs` → config condivisa → `app::build_router` → router health/root/public/internal → risposta JSON.

**Acceptance criteria**:

- [ ] `GET /health` restituisce `200` e JSON `{status: "ok", service: "bevymmo_gateway"}`.
- [ ] `GET /health/live` restituisce `200` senza contattare SpacetimeDB.
- [ ] `GET /` restituisce il messaggio di benvenuto.
- [ ] `GET /api/v1` identifica l'API pubblica v1.
- [ ] `GET /api/internal/v1` è raggiungibile solo attraverso il router interno, senza operazioni sensibili.
- [ ] Una route non esistente restituisce un errore JSON stabile.
- [ ] Il router può essere testato con `tower::ServiceExt` senza avviare un listener.
- [ ] Il processo usa `gateway.bind_addr` e termina in modo graceful su Ctrl+C/SIGTERM.

**RED**:

- Creare test di router che falliscono perché le route non esistono ancora.
- Verificare status code, content type e corpo JSON.
- Coprire almeno i mutanti probabili: route sbagliata, status `200` trasformato in `500`, campo `service` errato, path public/internal scambiati, fallback error che restituisce HTML.

**GREEN**:

- Estrarre `build_router` da `main.rs`.
- Separare `health.rs`, `public.rs`, `internal.rs` solo per i comportamenti effettivamente presenti.
- Introdurre uno state minimo condiviso se serve, senza aggiungere trait o dependency injection generica.
- Riutilizzare la configurazione già aggiunta in `bevymmo_app_support`.

**MUTATE**:

- Eseguire mutation testing sui moduli HTTP e registrare il report.

**KILL MUTANTS**:

- Rafforzare i test per ogni mutazione sopravvissuta che alteri status, route o payload.
- Chiedere conferma se sopravvive solo una mutazione cosmetica non osservabile dal contratto.

**REFACTOR**:

- Valutare solo duplicazione reale tra health e metadata response.
- Non introdurre un framework di response builder per due endpoint.

**Done when**: test HTTP, check, clippy e mutation report sono stati eseguiti e il criterio è approvato.

### Slice 2 — Il Gateway restituisce errori JSON coerenti

**Valore**: Angular e client developer possono gestire gli errori senza dipendere da testo libero o HTML generato dal framework.

**Attore**: client HTTP.

**Trigger**: route inesistente o errore applicativo controllato.

**Path**: handler/router → `AppError` → status HTTP → envelope JSON.

**Contratto proposto**:

```json
{
  "error": {
    "code": "not_found",
    "message": "Route not found",
    "request_id": "..."
  }
}
```

**Acceptance criteria**:

- [ ] Errori 404 hanno `Content-Type: application/json`.
- [ ] Il codice macchina (`code`) è stabile e distinto dal messaggio umano.
- [ ] Gli errori interni non espongono stack trace, SQL, token o dettagli sensibili.
- [ ] Esiste un request ID nei log e, se possibile, nella risposta.
- [ ] Il formato è documentato per il frontend.

**RED**:

- Testare 404 e almeno un errore applicativo controllato.
- Testare che il payload non contenga HTML o dettagli interni.
- Coprire mutanti su codice errore, status e presenza del request ID.

**GREEN**:

- Introdurre `error.rs` e un layer/mapping minimo compatibile con Axum.
- Usare una sola forma di errore per il Gateway.

**MUTATE / KILL MUTANTS / REFACTOR**:

- Eseguire mutation testing, aggiungere test per i sopravvissuti e rimuovere solo duplicazioni dimostrate.

**Done when**: tutti gli errori coperti hanno un contratto JSON testato e non espongono dettagli interni.

### Slice 3 — Il Gateway distingue configurazione e runtime senza introdurre autenticazione prematura

**Valore**: l'applicazione è pronta a configurare OIDC e integrazioni future senza mescolare segreti o parsing dentro gli handler.

**Attore**: operatore/deployer.

**Trigger**: avvio del Gateway con configurazione default o override environment.

**Path**: `Settings` → `GatewayConfig` → validazione → `AppState` → router.

**Acceptance criteria**:

- [ ] `gateway.bind_addr` continua a funzionare da `config/default.toml`.
- [ ] Un override environment valido cambia l'indirizzo senza modificare codice.
- [ ] Un bind address invalido fallisce all'avvio con messaggio chiaro.
- [ ] La configurazione futura OIDC ha nomi previsti (`issuer`, `audience`) ma non viene considerata abilitata se incompleta.
- [ ] Nessun secret viene stampato nei log.
- [ ] La configurazione non viene letta direttamente dagli handler.

**RED**:

- Testare deserializzazione default, override e configurazione invalida.
- Testare che configurazione OIDC incompleta non abiliti per errore l'autenticazione.
- Coprire mutanti su default address, validazione e flag di abilitazione.

**GREEN**:

- Estrarre `config.rs`/tipo `GatewayConfig` dal bootstrap.
- Passare allo state solo i valori necessari alle route.
- Lasciare l'autenticazione disabilitata e documentata.

**MUTATE / KILL MUTANTS / REFACTOR**:

- Eseguire mutation testing sui parser e sulla validazione.
- Non creare un sistema config generico oltre il bisogno attuale.

**Done when**: configurazione e runtime sono separati, testati e nessun login viene simulato.

### Slice 4 — Il Gateway ha un adapter SpacetimeDB testabile, senza esporre il database

**Valore**: il primo endpoint reale potrà usare SpacetimeDB senza accoppiare i router al client/database SDK.

**Attore**: futuro client del sito o developer.

**Trigger**: chiamata a un use case read-only scelto dopo aver verificato le tabelle disponibili.

**Nota**: questa slice richiede prima di confermare quale informazione minima sia utile e stabile. Non scegliere un endpoint arbitrario solo per dimostrare una query.

**Acceptance criteria**:

- [ ] Esiste un adapter con una API di dominio applicativo piccola e intenzionale.
- [ ] Il router chiama un service, non costruisce query direttamente.
- [ ] Il DTO pubblico è separato dalla row SpacetimeDB.
- [ ] Il test del service usa un fake adapter o fixture deterministica.
- [ ] Il test di integrazione opzionale usa un'istanza SpacetimeDB locale e non è richiesto per i test unitari.
- [ ] Errori SpacetimeDB vengono convertiti nel contratto `AppError`.
- [ ] Non è possibile passare SQL o nome reducer arbitrario dall'HTTP request.

**RED**:

- Definire il test del primo use case read-only scelto.
- Testare successo, assenza dati e errore adapter.
- Coprire mutanti che restituiscono dati di un altro utente, nascondono un errore o bypassano il service.

**GREEN**:

- Implementare il minimo adapter/service necessario.
- Mantenere la dipendenza SpacetimeDB confinata a `adapters/spacetimedb.rs`.
- Aggiungere endpoint solo dopo aver definito il contratto JSON.

**MUTATE / KILL MUTANTS / REFACTOR**:

- Mutation testing su service e mapping DTO.
- Rafforzare autorizzazione e ownership se i dati sono user-scoped.

**Done when**: un endpoint read-only reale percorre HTTP → service → adapter → SpacetimeDB/fake, con errori e DTO testati.

### Slice 5 — Decisione autenticazione e primo endpoint user-scoped

**Valore**: il sito può identificare l'utente senza duplicare il login in Postgres e SpacetimeDB.

**Prerequisito**: approvazione esplicita del provider OIDC e del contratto dei claim.

**Attore**: utente autenticato del sito.

**Trigger**: richiesta HTTP con `Authorization: Bearer <JWT>`.

**Path**: bearer extractor → verifica firma/issuer/audience → claims → service → Identity/subject SpacetimeDB → DTO.

**Acceptance criteria**:

- [ ] Token assente restituisce `401`.
- [ ] Token malformato o con firma invalida restituisce `401`.
- [ ] Issuer o audience errati restituiscono `401`.
- [ ] Un token valido produce un'identità applicativa esplicita.
- [ ] Il modulo SpacetimeDB valida lo stesso issuer/audience prima di fidarsi dei claim.
- [ ] Il primo endpoint user-scoped non permette di leggere dati di un altro utente cambiando un ID nell'URL.
- [ ] Nessuna password viene salvata nel repository o nel Gateway.

**RED / GREEN / MUTATE / KILL MUTANTS / REFACTOR**:

- Da eseguire solo dopo la scelta del provider.
- I mutanti più importanti sono bypass dell'issuer, bypass dell'audience, accettazione del token assente e mancata verifica dell'ownership.

**Done when**: autenticazione reale e ownership sono testate end-to-end con il provider scelto.

## Sequenza consigliata di implementazione

1. Approvare questo piano e i criteri della Slice 1.
2. Completare Slice 1: router modulare e test HTTP.
3. Completare Slice 2: error envelope.
4. Completare Slice 3: configurazione pulita.
5. Scegliere il primo endpoint read-only e completare Slice 4.
6. Decidere il provider OIDC prima di iniziare Slice 5.
7. Solo dopo Slice 5 rivalutare se esiste un requisito concreto per Postgres.

## Quality gate

Prima di chiudere ogni slice:

```sh
cargo fmt --all -- --check
cargo test -p bevymmo_gateway
cargo check -p bevymmo_gateway
cargo clippy -p bevymmo_gateway --all-targets -- -D warnings
```

Prima del merge del milestone:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

In aggiunta:

- report mutation testing;
- verifica manuale con `curl` su `/`, `/health`, `/health/live` e `/api/v1`;
- verifica che il processo termini su Ctrl+C;
- verifica che nessuna configurazione locale/segreta venga committata.

## Domande aperte prima della Slice 4/5

1. Quale endpoint read-only è davvero prioritario per il sito: stato account, personaggi, catalogo mondo o news?
2. Vuoi usare SpacetimeAuth oppure hai già un provider OAuth/OIDC preferito?
3. Le API developer devono leggere dati di gameplay o fornire solo integrazioni/eventi?
4. Il Gateway sarà esposto pubblicamente dietro reverse proxy, oppure sarà usato solo localmente nella prima fase?

---

*Questo file va eliminato quando il piano è completato; le decisioni architetturali stabili vanno poi documentate in un ADR o nella documentazione del progetto.*
