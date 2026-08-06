# Piano Fix: Fireball Damage + Visual Replica

## Sintomi

1. **Click Q → vedo fireball, ma nessuno perde vita**
2. **Player1 casta fireball → Player2 non vede nulla**

## Diagnosi

### Bug 1: `targets_query` esclude Dummy (e future entity)

In `src/plugins/spells/systems.rs`, `process_cast_requests`:

```rust
targets_query: Query<(Entity, &Position), Or<(With<Player>, With<Enemy>)>>,
```

Il Dummy ha marker `Dummy`, **non** `Player` né `Enemy`. Quindi non appare mai in `potential_targets`. La fireball esplode sulla sua posizione ma trova zero target → zero `DamageEvent` → zero danno.

Anche tutti i futuri tipi di entity (NPC, boss, oggetti distruttibili) sarebbero esclusi.

### Bug 2: `FireballVisualEffect` è locale, non replicata

In `src/network/client.rs`, `cast_fireball_on_key`:

```rust
visuals.write(FireballVisualEffect { start, end: target });
```

`FireballVisualEffect` è registrato come **messaggio locale** in `src/plugins/spells/effects.rs`:

```rust
app.add_message::<FireballVisualEffect>();
```

**Non** è un messaggio di rete Lightyear. È solo un bus ECS interno al singolo processo. Quindi:

- il client che preme Q vede la fireball (perché scrive nel bus locale)
- il server **non** invia alcun effetto visivo agli altri client
- Player2 non riceve nulla → non vede la fireball

Il server processa lo spell (danno, cooldown) ma **non broadcasta** nulla di visivo.

## Fix

### Fix 1: generalizzare `targets_query`

**File:** `src/plugins/spells/systems.rs`

Cambiare:

```rust
targets_query: Query<(Entity, &Position), Or<(With<Player>, With<Enemy>)>>,
```

in:

```rust
targets_query: Query<(Entity, &Position, &VitalStats), With<GameEntity>>,
```

Vantaggi:

- include Player, Enemy, **Dummy**, e qualsiasi futura entity con `GameEntity`
- `VitalStats` in query permette di filtrare target morti direttamente (`is_dead()`)
- `With<GameEntity>` è il marker generico corretto già usato dal resto della codebase

Modifiche nel corpo di `process_cast_requests`:

```rust
let potential_targets: Vec<(Entity, Vec3)> = targets_query
    .iter()
    .filter(|(_, _, vital)| !vital.is_dead())
    .map(|(entity, pos, _)| (entity, pos.0))
    .collect();
```

### Fix 2: replicare il visual effect via network

Serve che il server broadcasti l'effetto visivo a tutti i client dopo aver processato lo spell.

#### Approccio

Il server invia un messaggio di rete con start/end della fireball a tutti i client. Ogni client (incluso il caster) riceve il messaggio e spawna il visual.

#### File coinvolti

- `src/network/protocol.rs` — nuovo messaggio rete
- `src/plugins/spells/effects.rs` — ricezione messaggio rete + spawn visual
- `src/plugins/spells/systems.rs` — server invia messaggio dopo cast
- `src/plugins/spells/plugin.rs` — registrazione messaggio rete
- `src/network/client.rs` — rimuovere scrittura locale del visual nel `cast_fireball_on_key`

#### Dettagli

**1. Nuovo messaggio rete in `protocol.rs`:**

```rust
/// Messaggio server -> client per replicare un effetto visivo spell.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SpellVisualEffect {
    pub spell_id: String,
    pub start: Vec3,
    pub end: Vec3,
}
```

Registrazione nel `ProtocolPlugin`:

```rust
app.register_message::<SpellVisualEffect>()
    .add_direction(NetworkDirection::ServerToClient);
```

**2. Server: invia visual dopo cast in `process_cast_requests`:**

Dopo `spell.cast(&mut ctx)` e dopo aver drenato i damage events:

```rust
// Broadcast visual effect to all clients
for visual_sender in visual_senders.iter_mut() {
    visual_sender.send::<Channel1>(SpellVisualEffect {
        spell_id: request.spell_id.as_str().to_string(),
        start: caster_position,
        end: ctx.effective_center(), // o request.target_position
    });
}
```

Serve aggiungere alla firma di `process_cast_requests`:

```rust
mut visual_senders: Query<&mut MessageSender<SpellVisualEffect>, ...>,
```

Oppure usare `ServerMultiMessageSender` come fa già `send_messages` in `server.rs`.

**3. Client: ricezione in `effects.rs`:**

Aggiungere un sistema che legge `MessageReader<SpellVisualEffect>` (messaggio rete) e spawna il visual, in parallelo al sistema esistente che legge `FireballVisualEffect` (locale).

Oppure più pulito: unificare. Il sistema `spawn_fireball_visuals` legge direttamente `MessageReader<SpellVisualEffect>` invece di `MessageReader<FireballVisualEffect>`.

**4. Rimuovere scrittura locale in `client.rs`:**

In `cast_fireball_on_key`, rimuovere:

```rust
visuals.write(FireballVisualEffect { start, end: target });
```

Il visual ora arriva solo dal server, garantendo che tutti i client (incluso il caster) vedano la stessa cosa.

#### Trade-off latency

Il caster vedrà la fireball con un piccolo ritardo di rete (1 tick ~ 16ms a 60Hz). Per ora è accettabile. In futuro si può aggiungere client prediction del visual.

## Fase 3 — Spell Followball (Homing Projectile)

### Obiettivo

Una nuova spell "Followball" che spawna un proiettile che insegue il target selezionato (`CurrentTarget`). Il proiettile è una entity replicata, quindi tutti i client la vedono. Al contatto, applica `DamageEvent` e despawn.

### Architettura

Il trait `Spell::cast()` non ha accesso a `Commands`. Seguendo il pattern esistente di `pending_damage`/`pending_healing`, aggiungo `pending_projectiles` al `SpellCastContext`.

```
Client: preme E
  → legge CurrentTarget.entity
  → invia SpellCastCommand { spell_id: "followball", target_entity }
Server: riceve comando
  → SpellCastRequest { caster, spell_id, target_entity }
  → process_cast_requests
     → FollowballSpell.cast(ctx)
        → ctx.emit_projectile(target, speed, damage)
     → drain pending_projectiles → spawn HomingProjectile entity (replicata)
  → sistema update_homing_projectiles (FixedUpdate, server)
     → muove proiettile verso target
     → se hit → DamageEvent + despawn
     → se target morto/despawnato → despawn projectile
Client: vede proiettile perché Position/EntityColor replicate
```

### File nuovi

- `src/spells/followball/mod.rs`
- `src/spells/followball/definition.rs`
- `src/plugins/spells/projectiles.rs` (component + system)

### File da modificare

- `src/network/protocol.rs` — `target_entity` in `SpellCastCommand`, `SpellVisualEffect`, `MapEntities`
- `src/plugins/spells/context.rs` — `target_entity`, `pending_projectiles`, `emit_projectile`
- `src/plugins/spells/systems.rs` — `targets_query` fix, drain projectiles, broadcast visual
- `src/plugins/spells/plugin.rs` — registra sistema homing
- `src/plugins/spells/mod.rs` — re-export
- `src/plugins/spells/effects.rs` — ricezione visual da rete
- `src/plugins/key_mapping.rs` — `cast_followball: KeyCode::KeyE`
- `src/plugins/entity/player/spawn.rs` — aggiungi followball a spellbook
- `src/plugins/spells/ui.rs` — aggiungi followball a HUD
- `src/network/client.rs` — handler cast followball con target_entity
- `src/network/server.rs` — passa target_entity
- `src/spells/mod.rs` — modulo followball
- `src/plugins/spells/systems.rs` — registra FollowballSpell in `register_builtin_spells`

### Dettagli tecnici

#### `SpellCastCommand` con `target_entity`

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SpellCastCommand {
    pub spell_id: String,
    pub target_position: Option<Vec3>,
    pub target_entity: Option<Entity>,
}

impl MapEntities for SpellCastCommand {
    fn map_entities<M: EntityMapper>(&mut self, mapper: &mut M) {
        if let Some(ref mut e) = self.target_entity {
            *e = mapper.map_entity(*e);
        }
    }
}
```

#### `SpellCastContext` con projectile

```rust
pub struct ProjectileSpawnRequest {
    pub target: Entity,
    pub speed: f32,
    pub damage: f32,
    pub hit_radius: f32,
}

// in SpellCastContext:
pub target_entity: Option<Entity>,
pub pending_projectiles: Vec<ProjectileSpawnRequest>,
```

#### `HomingProjectile` component

```rust
#[derive(Component)]
pub struct HomingProjectile {
    pub target: Entity,
    pub speed: f32,
    pub damage: f32,
    pub hit_radius: f32,
}
```

La entity projectile viene spawnta con `Position` + `EntityColor` + `Replicate` (come le altre GameEntity). Il renderer esistente genera automaticamente un cubo.

#### FollowballSpell

```rust
pub struct FollowballSpell;
impl Spell for FollowballSpell {
    fn cast(&self, ctx: &mut SpellCastContext) {
        if let Some(target) = ctx.target_entity {
            ctx.emit_projectile(target, Self::SPEED, ctx.caster_combat.attack_power * 1.2);
        }
    }
}
```

#### Sistema homing (server, FixedUpdate)

```rust
fn update_homing_projectiles(
    mut projectiles: Query<(Entity, &mut Position, &HomingProjectile)>,
    targets: Query<&Position, With<GameEntity>>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut commands: Commands,
) {
    for (proj_entity, mut proj_pos, proj) in projectiles.iter_mut() {
        let Ok(target_pos) = targets.get(proj.target) else {
            commands.entity(proj_entity).despawn();
            continue;
        };
        let direction = target_pos.0 - proj_pos.0;
        let distance = direction.length();
        if distance <= proj.hit_radius {
            damage_events.write(DamageEvent {
                target: proj.target,
                source: None, // TODO: salvare caster nel projectile
                amount: proj.damage,
            });
            commands.entity(proj_entity).despawn();
            continue;
        }
        proj_pos.0 += direction.normalize() * proj.speed;
    }
}
```

## Ordine implementativo completo

1. Fix `targets_query` (Bug 1)
2. `SpellVisualEffect` messaggio rete + server broadcast (Bug 2)
3. `target_entity` in protocollo + `MapEntities`
4. `SpellCastContext` estensione con `pending_projectiles`
5. `HomingProjectile` + sistema movement/hit
6. `FollowballSpell` definition + registration
7. Key binding KeyE + client handler con `CurrentTarget`
8. Player spellbook + HUD
9. `cargo check` + `cargo test`
