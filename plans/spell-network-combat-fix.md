# Piano: fix spell Q/E, danni e visual networked

## Obiettivo

Correggere il comportamento delle spell in multiplayer:

- `Q` / Fireball: non usa il target selezionato. Parte dal player, usa la direzione in cui il player sta guardando e colpisce la prima entità viva davanti entro range.
- `E` / Followball: usa il target selezionato e crea un projectile homing server-authoritative che segue quel target.
- Player1 e Player2 devono vedere gli stessi effetti rilevanti tramite server/network, non tramite effetti solo locali.

## Problemi trovati

1. `SpellVisualEffect` esiste ma non viene registrato/inviato come messaggio server -> client.
2. Fireball attuale è AoE su `target_position`; non implementa "prima cosa davanti al player".
3. Followball manda `Entity` client-side al server (`target_entity`), ma gli handle ECS Bevy non sono un identificatore stabile tra client e server.
4. Il projectile Followball viene replicato solo come `Position + EntityColor`, quindi il renderer lo tratta come entità generica.
5. Il cooldown HUD parte lato client appena premi il tasto, anche se il server può rifiutare il cast. Questo rimane un follow-up consigliato.

## Design target

### Fireball

Pipeline:

```text
Client Player1 preme Q
  -> invia SpellCastCommand { spell_id: "fireball", target_id: None, target_position: None }
Server
  -> risolve caster dal peer
  -> legge Position + LookDirection autorevoli
  -> cerca la prima GameEntity viva davanti entro CAST_RANGE e HIT_RADIUS
  -> applica DamageEvent al primo hit
  -> invia SpellVisualEffect a tutti con start/end
Client Player1 e Player2
  -> ricevono SpellVisualEffect
  -> spawnano/animano la fireball locale
  -> vedono HP aggiornati tramite VitalStats replicato
```

### Followball

Pipeline:

```text
Client Player1 seleziona un target
  -> CurrentTarget contiene entity locale
  -> legge NetworkEntityId replicato dal target
Client Player1 preme E
  -> invia SpellCastCommand { spell_id: "followball", target_id: Some(id) }
Server
  -> risolve target_id -> Entity server-side
  -> crea HomingProjectile verso quell'Entity
  -> muove il projectile in FixedUpdate
  -> se raggiunge il target, emette DamageEvent
Client Player1 e Player2
  -> vedono projectile replicato/renderizzato come visual spell
  -> vedono HP aggiornati tramite VitalStats replicato
```

## Modifiche implementative

### 1. Protocollo

File: `src/network/protocol.rs`

- Aggiungere componente replicato stabile:

```rust
pub struct NetworkEntityId(pub u64);
```

- Cambiare `SpellCastCommand`:

```rust
pub target_id: Option<u64>
```

invece di affidarsi a `Entity` client-side.

- Registrare `SpellVisualEffect` server -> client.
- Aggiungere `ProjectileVisual` replicato per distinguere i projectile dal renderer generico.

### 2. ID stabili entity

File: `src/plugins/entity/spawn.rs`

- Aggiungere `NetworkEntityId` al `GameEntityBundle`.
- Usare una `AtomicU64` server-side semplice per assegnare ID unici a tutte le entità create da `GameEntityBundle`.

### 3. Fireball hitscan frontale

File:

- `src/plugins/spells/context.rs`
- `src/plugins/spells/systems.rs`
- `src/spells/fireball/definition.rs`
- `src/network/client.rs`

Modifiche:

- Aggiungere `caster_look_direction` al `SpellCastContext`.
- Il server legge `LookDirection` dalla query caster.
- Il client non manda più `target_position` per Fireball.
- Fireball cerca il primo target davanti:
  - `forward_distance = dot(to_target, direction)`
  - scarta dietro/fuori range
  - calcola distanza laterale dalla linea di tiro
  - tiene il target più vicino lungo la linea.
- Fireball emette `SpellVisualEffect` con end = punto colpito o range massimo.

### 4. Followball con target stabile

File:

- `src/network/client.rs`
- `src/network/server.rs`
- `src/spells/followball/definition.rs`

Modifiche:

- Il client legge `NetworkEntityId` dal target selezionato.
- Il server risolve `target_id` nella propria `Entity`.
- La spell continua a generare `HomingProjectile` su un target server-side valido.

### 5. Projectile visual

File:

- `src/network/protocol.rs`
- `src/plugins/spells/systems.rs`
- `src/plugins/renderer.rs`

Modifiche:

- `spawn_homing_projectile` inserisce `ProjectileVisual`.
- Il renderer usa mesh/materiale diversi per projectile, più piccoli ed emissivi.

## Validazione

1. `cargo test`
2. `cargo check`
3. Test manuale multiplayer:
   - avvia server/host
   - avvia Player1 e Player2
   - Player1 preme `Q` guardando un enemy/dummy entro range: entrambi vedono visual e HP scendere.
   - Player1 seleziona enemy/dummy e preme `E`: entrambi vedono followball seguire il target e HP scendere all'impatto.

## Follow-up consigliati

- Spostare cooldown HUD su conferma server (`SpellCastAccepted`) invece che avvio immediato client-side.
- Aggiungere team/faction filtering per evitare che Fireball colpisca altri player o entità amiche, se non desiderato.
- Migliorare animazioni/materiali per Fireball e Followball.
