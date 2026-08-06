# Piano Select / Target / Dummy / Spell Follow

## Obiettivo

Implementare un sistema MMO-style per:

1. selezionare una entity con tasto destro senza muovere il player;
2. visualizzare il target selezionato con un cerchio rosso sotto l'entity;
3. mostrare le stats già esistenti del target nella UI;
4. colorare le healthbar in base al tipo di entity;
5. aggiungere una nuova entity `Dummy`, ferma, con `1_000_000_000` HP, utile per testare danni;
6. aggiungere una spell/proiettile che segue il target selezionato.

Il piano deve appoggiarsi alla codebase esistente, senza reinventare componenti già presenti.

## Vincoli

- Non creare un nuovo componente `Stats`.
- Usare i componenti già esistenti:
  - `MovementStats`
  - `CombatStats`
  - `VitalStats`
  - `StatsBundleData`
- Non aggiungere un sistema di livelli.
- Il tasto sinistro continua a gestire il movimento punta-e-clicca.
- Il tasto destro seleziona un target e non deve cambiare `MoveTarget`.
- Il cerchio di selezione deve stare nella UI/visual feedback, preferibilmente sotto `src/ui/`.
- I danni devono continuare a passare da `DamageEvent` e dai sistemi in `src/stats/systems.rs`.
- La simulazione gameplay deve restare server-authoritative dove già lo è.

## Stato attuale rilevante della codebase

### Stats

File:

- `src/stats/components.rs`
- `src/stats/defaults.rs`
- `src/stats/events.rs`
- `src/stats/systems.rs`

Esistono già:

```rust
MovementStats
CombatStats
VitalStats
StatsBundleData
DamageEvent
HealEvent
```

`VitalStats` contiene già:

```rust
pub struct VitalStats {
    pub current_health: f32,
    pub max_health: f32,
    pub max_mana: f32,
    pub mana_regeneration: f32,
}
```

Quindi target frame, dummy e spell devono usare questo componente.

### Entity spawn

File:

- `src/plugins/entity/components.rs`
- `src/plugins/entity/definition.rs`
- `src/plugins/entity/spawn.rs`
- `src/plugins/entity/player/spawn.rs`
- `src/plugins/entity/enemy/spawn.rs`

Esiste già il pattern:

```rust
EntityDefinition
spawn_entity::<T>()
GameEntityBundle
```

Ogni nuova entity concreta deve usare questo meccanismo.

### Movimento

File:

- `src/plugins/player_movement.rs`

Il movimento usa già solo:

```rust
MouseButton::Left
```

Quindi il sistema target può usare `MouseButton::Right` senza toccare il movimento base.

### UI entity bar

File:

- `src/ui/entity_bar/components.rs`
- `src/ui/entity_bar/plugin.rs`
- `src/ui/entity_bar/systems.rs`

Esiste già una UI flottante sopra le entity con `Position` + `VitalStats`.
Va estesa per colore healthbar in base a `EntityKind`, non duplicata.

### Spell

File:

- `src/plugins/spells/events.rs`
- `src/plugins/spells/systems.rs`
- `src/plugins/spells/context.rs`
- `src/spells/fireball/definition.rs`
- `src/network/protocol.rs`
- `src/network/client.rs`
- `src/network/server.rs`

Attualmente `SpellCastCommand` e `SpellCastRequest` usano `target_position: Option<Vec3>`.
La spell follow richiederà anche un target entity o un identificatore equivalente.

## Fase 1 — Aggiungere `EntityKind`

### File da modificare

- `src/plugins/entity/components.rs`
- `src/plugins/entity/definition.rs`
- `src/plugins/entity/spawn.rs`
- `src/plugins/entity/player/spawn.rs`
- `src/plugins/entity/enemy/spawn.rs`
- `src/network/protocol.rs`

### Nuovo enum

In `src/plugins/entity/components.rs`:

```rust
#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq, Eq)]
#[reflect(Component)]
pub enum EntityKind {
    Player,
    Friendly,
    Neutral,
    Hostile,
}
```

### Integrazione con `EntityDefinition`

Aggiungere default:

```rust
fn entity_kind() -> EntityKind {
    EntityKind::Neutral
}
```

### Integrazione con `GameEntityBundle`

Aggiungere campo:

```rust
entity_kind: EntityKind,
```

Passare `T::entity_kind()` da `spawn_entity::<T>()`.

### Override

Player:

```rust
fn entity_kind() -> EntityKind {
    EntityKind::Player
}
```

Enemy:

```rust
fn entity_kind() -> EntityKind {
    EntityKind::Hostile
}
```

### Replicazione

In `src/network/protocol.rs` registrare:

```rust
app.component::<EntityKind>().replicate();
```

## Fase 2 — Aggiungere entity `Dummy`

### File nuovi

- `src/plugins/entity/dummy/mod.rs`
- `src/plugins/entity/dummy/components.rs`
- `src/plugins/entity/dummy/spawn.rs`

### File da modificare

- `src/plugins/entity/mod.rs`
- `src/stats/defaults.rs`
- `src/network/server.rs` o il punto di spawn demo attuale

### Component

```rust
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Dummy;
```

### Default stats

In `src/stats/defaults.rs`:

```rust
pub fn dummy_defaults() -> StatsBundleData {
    StatsBundleData {
        movement: MovementStats { speed: 0.0 },
        combat: CombatStats {
            attack_power: 0.0,
            armor: 0.0,
        },
        vital: VitalStats {
            current_health: 1_000_000_000.0,
            max_health: 1_000_000_000.0,
            max_mana: 0.0,
            mana_regeneration: 0.0,
        },
    }
}
```

### EntityDefinition

```rust
impl EntityDefinition for Dummy {
    fn name() -> &'static str {
        "Dummy"
    }

    fn bundle() -> impl Bundle {
        (
            Dummy,
            PlayerName("Dummy".to_string()),
        )
    }

    fn initial_position() -> Vec3 {
        Vec3::new(8.0, 0.0, 0.0)
    }

    fn initial_color() -> Color {
        Color::srgb(0.7, 0.1, 0.1)
    }

    fn entity_kind() -> EntityKind {
        EntityKind::Hostile
    }

    fn stats() -> StatsBundleData {
        crate::stats::defaults::dummy_defaults()
    }
}
```

### Plugin

Registrare `dummy::DummyPlugin` dentro `src/plugins/entity/mod.rs`.

Il Dummy deve essere fermo e non avere AI. Non aggiungere `AggroRange` e non aggiungere spellbook.

## Fase 3 — Targeting con tasto destro

### File nuovi consigliati

- `src/plugins/targeting/mod.rs`
- `src/plugins/targeting/plugin.rs`
- `src/plugins/targeting/resources.rs`
- `src/plugins/targeting/systems.rs`

### Resource

```rust
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct CurrentTarget {
    pub entity: Option<Entity>,
}
```

### Input

- `MouseButton::Right`: prova a selezionare una entity.
- `MouseButton::Left`: resta movimento, già gestito da `PlayerMovementPlugin`.
- `Escape`: clear target.

### Picking iniziale

Per non introdurre subito collider/picking plugin:

1. creare ray da `Camera3d` e cursore;
2. query entity con `GameEntity`, `Position`, `VitalStats`, `EntityKind`;
3. usare un test ray-sphere semplice con raggio iniziale circa `1.2`;
4. scegliere la entity più vicina lungo il ray;
5. ignorare target morti con `VitalStats::is_dead()`.

### Query targettabili iniziale

```rust
Query<(Entity, &Position, &VitalStats, &EntityKind), With<GameEntity>>
```

Non serve un componente `Targetable` nella prima iterazione: tutte le `GameEntity` vive con `VitalStats` sono targettabili. Se in futuro serviranno oggetti non targettabili, aggiungere marker dedicato.

## Fase 4 — Cerchio rosso sotto il target

### Path richiesto

Mettere in `src/ui/`.

File consigliati:

- `src/ui/target_indicator/mod.rs`
- `src/ui/target_indicator/plugin.rs`
- `src/ui/target_indicator/components.rs`
- `src/ui/target_indicator/systems.rs`

### Componenti

```rust
#[derive(Component)]
pub struct TargetSelectionRing;

#[derive(Component)]
pub struct TargetSelectionRingTarget(pub Entity);
```

### Comportamento

- Se `CurrentTarget.entity` è `Some(target)`, mostra un solo ring rosso.
- Il ring segue la `Position` del target.
- Se il target sparisce o perde `Position`, despawn del ring e clear target.
- Se il target cambia, il ring viene spostato/ricreato per il nuovo target.

### Mesh

Riutilizzare pattern simile a `spawn_click_indicator` in `src/plugins/player_movement.rs`:

- mesh `Torus`
- rotazione su piano XZ
- posizione `target_position + Vec3::Y * 0.04`
- materiale rosso/emissive rosso

## Fase 5 — Colorare healthbar in base a `EntityKind`

### File da modificare

- `src/ui/entity_bar/components.rs`
- `src/ui/entity_bar/systems.rs`
- opzionale `src/ui/theme.rs`

### Strategia

Estendere `update_floating_ui_content` per leggere anche `EntityKind`:

```rust
target_query: Query<(&VitalStats, Option<&PlayerName>, Option<&EntityKind>)>
```

Aggiornare `BackgroundColor` del nodo `hp_fill`, referenziato da `EntityBarParts`.

### Colori iniziali

- `EntityKind::Player`: verde/blu-verde
- `EntityKind::Friendly`: verde
- `EntityKind::Neutral`: giallo
- `EntityKind::Hostile`: rosso
- `None`: fallback a `theme.hp_fill`

## Fase 6 — Target frame UI

### File nuovi consigliati

- `src/ui/target_frame/mod.rs`
- `src/ui/target_frame/plugin.rs`
- `src/ui/target_frame/components.rs`
- `src/ui/target_frame/systems.rs`

### Dati da mostrare

- nome: `PlayerName` se presente, fallback `"Entity"`;
- HP: `VitalStats.current_health / VitalStats.max_health`;
- tipo: `EntityKind`;
- opzionale: `CombatStats.attack_power` / `CombatStats.armor` se utile per debug.

Non mostrare livelli.

### Comportamento

- Nessun target: frame nascosto.
- Target valido: frame visibile e aggiornato.
- Target despawnato/morto: clear target e nascondi frame.

## Fase 7 — Spell follow sul target

### Problema tecnico da verificare

Attualmente i comandi spell sono position-based:

```rust
pub struct SpellCastCommand {
    pub spell_id: String,
    pub target_position: Option<Vec3>,
}
```

Per una spell follow serve anche il target entity:

```rust
pub struct SpellCastCommand {
    pub spell_id: String,
    pub target_position: Option<Vec3>,
    pub target_entity: Option<Entity>,
}
```

Lo stesso vale per `SpellCastRequest`.

Va verificato se Lightyear gestisce correttamente `Entity` nei messaggi client/server in questa configurazione. Se non basta, introdurre un id stabile di gameplay/network per target entity.

### File probabilmente coinvolti

- `src/network/protocol.rs`
- `src/network/client.rs`
- `src/network/server.rs`
- `src/plugins/spells/events.rs`
- `src/plugins/spells/context.rs`
- `src/plugins/spells/systems.rs`
- `src/plugins/spells/projectiles.rs`
- `src/spells/homing_fireball/mod.rs`
- `src/spells/homing_fireball/definition.rs`
- `src/spells/mod.rs`

### Nuovo componente projectile

```rust
#[derive(Component)]
pub struct HomingProjectile {
    pub caster: Entity,
    pub target: Entity,
    pub speed: f32,
    pub damage: f32,
    pub hit_radius: f32,
}
```

### Sistema server-authoritative

Ogni tick:

1. leggere `Position` projectile;
2. leggere `Position` target;
3. muovere projectile verso target;
4. se `distance <= hit_radius`, inviare `DamageEvent` e despawn projectile;
5. se target despawnato/morto, despawn projectile.

### Visualizzazione

Se il projectile è una entity replicata con `Position` + `EntityColor`, il renderer esistente può già generare una mesh cubo. In futuro si potrà specializzare il renderer per projectile.

### Nuova spell

Nuova spell consigliata:

```text
homing_fireball
```

Non modificare subito la `fireball` AoE esistente.

Il player spellbook va aggiornato in:

```rust
src/plugins/entity/player/spawn.rs
```

aggiungendo `SpellId::new("homing_fireball")`.

## Ordine implementativo consigliato

1. `EntityKind` replicato e inserito nello spawn centralizzato.
2. `Dummy` come nuova entity concreta e spawn demo lato server.
3. `CurrentTarget` + right click picking.
4. Cerchio rosso in `src/ui/target_indicator`.
5. Healthbar colorate in `src/ui/entity_bar`.
6. Target frame in `src/ui/target_frame`.
7. Spell follow con `target_entity` e projectile server-authoritative.

## Test/validazione consigliata

Dopo ogni fase:

```sh
cargo test
cargo check
```

Test mirati utili:

- `GameEntityBundle` contiene `EntityKind`.
- `dummy_defaults()` ha `1_000_000_000` HP e speed `0.0`.
- healthbar sceglie colore corretto per ogni `EntityKind`.
- `CurrentTarget` viene pulito se il target sparisce.
- projectile emette `DamageEvent` quando raggiunge il target.

## Note finali

Il sistema deve restare minimale e coerente con la codebase attuale. Non introdurre livelli, non duplicare stats, non creare sistemi di danno paralleli. La prima iterazione può usare picking geometrico semplice ray-sphere; collider/picking plugin potranno arrivare dopo se necessario.
