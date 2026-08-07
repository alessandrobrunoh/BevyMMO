# Bevy Observers & Component Hooks Architecture Plan

## 1. Overview & Objectives

This document outlines the architectural strategy and implementation plan for adopting **Bevy Observers** (`Observer` / `On<Trigger>`) and **Component Hooks** (`on_add`, `on_remove`, etc.) in `BevyMMO`.

Transitioning from continuous frame-by-frame polling queries (`Query<(Entity, &Component), Without<Other>>` in `Update` systems) to a **push-based reactive architecture** will:
1. **Reduce CPU Overheads**: Eliminate redundant per-frame queries across hundreds of replicated entities.
2. **Eliminate Race Conditions**: Trigger immediate, synchronous event handling when state changes occur.
3. **Decouple Crates**: Allow presentation and UI systems to react to network/gameplay events without hard-coupling system schedules.
4. **Enforce Invariants**: Use Component Hooks to guarantee structural component dependencies at the storage layer.

---

## 2. Core Concepts: Observers vs. Component Hooks

| Feature | Observers (`Observer` / `On<Trigger>`) | Component Hooks (`on_add`, `on_remove`) |
| :--- | :--- | :--- |
| **Execution Trigger** | Emitted `Trigger<T>` or lifecycle events (`On<Add, C>`, `On<Remove, C>`, `On<Despawn>`). | Storage-level component insertion/removal (`on_add`, `on_insert`, `on_replace`, `on_remove`). |
| **System Access** | Full Bevy System parameter support (`Commands`, `Query`, `Res`, `ResMut`, `EventWriter`). | `DeferredWorld`, `Entity`, `ComponentId`. Limited ECS mutations via deferred world. |
| **Primary Use Case** | Gameplay events, targeted damage/effects, network lifecycle, UI state transitions, visual spawning. | Data integrity, automatic component attachment, low-level spatial/index maintenance. |
| **Location in Code** | Registered on `App` (`app.add_observer(...)`) or attached to specific entities. | Attached to `Component` definitions via derive macros or `.with_hooks()`. |

---

## 3. Current Baseline in BevyMMO

`BevyMMO` already utilizes observers in [`crates/client/src/network/client.rs`](file:///C:/Users/alexb/Documents/Coding/Rust/BevyMMO/crates/client/src/network/client.rs#L54-L58) for networking lifecycle management:

```rust
app.add_observer(handle_connected);
app.add_observer(handle_disconnected);
app.add_observer(handle_predicted_spawn);
app.add_observer(handle_controlled_spawn);
app.add_observer(handle_interpolated_spawn);
```

While networking connection handling relies on observers, gameplay systems (such as spell casting, movement prediction, visual mesh instantiation, and UI updates) still rely on polling in `Update` systems.

---

## 4. Proposed Architectural Improvements

### Phase 1: Replicated Entity Lifecycle & Visual Presentation (`bevymmo_presentation`)

#### Current Problem
The client presentation crate frequently polls queries in `Update` to check if a replicated entity has received network components but lacks local visual representations (e.g. `Mesh3d`, `Transform`, `CastBarUI`).

#### Observer-Driven Solution
Replace polling with lifecycle observers registered on Lightyear replication tags (`Controlled`, `Predicted`, `Interpolated`):

1. **Entity Visual Instantiation**:
   ```rust
   // Triggered immediately when Lightyear adds `Predicted` or `Interpolated` to a replicated entity
   pub fn spawn_entity_visuals(
       trigger: On<Add, Predicted>,
       query: Query<&Archetype, With<Predicted>>,
       mut commands: Commands,
       assets: Res<GameAssets>,
   ) {
       let entity = trigger.entity();
       // Attach mesh, material, and spatial components based on entity archetype
       commands.entity(entity).insert((
           Mesh3d(assets.player_mesh.clone()),
           MeshMaterial3d(assets.player_material.clone()),
       ));
   }
   ```

2. **Entity Despawn & Cleanup**:
   ```rust
   pub fn cleanup_entity_visuals(
       trigger: On<Remove, Predicted>,
       mut commands: Commands,
   ) {
       // Clean up child entities, particle emitters, or floating health bars
   }
   ```

---

### Phase 2: Reactive Combat & Targeted Events (`bevymmo_shared` & `bevymmo_server`)

#### Current Problem
Combat events (damage, spell impacts, status effects) are either queued in global `Events<T>` or processed by iterating over all entities every tick on the server.

#### Observer-Driven Solution
Use targeted `Trigger` events for point-to-point combat interactions:

1. **Event Contracts (`bevymmo_shared`)**:
   ```rust
   #[derive(Event, Clone, Debug, Serialize, Deserialize)]
   pub struct DamageDealt {
       pub attacker: Entity,
       pub amount: f32,
       pub is_crit: bool,
   }

   #[derive(Event, Clone, Debug)]
   pub struct SpellCastCompleted {
       pub spell_id: u32,
       pub target: Option<Entity>,
   }
   ```

2. **Server Damage Resolution (`bevymmo_server`)**:
   ```rust
   pub fn on_damage_dealt(
       trigger: On<DamageDealt>,
       mut health_query: Query<&mut Health>,
       mut commands: Commands,
   ) {
       let target = trigger.entity();
       let damage = trigger.event();

       if let Ok(mut health) = health_query.get_mut(target) {
           health.current = (health.current - damage.amount).max(0.0);
           
           if health.current == 0.0 {
               commands.trigger_targets(EntityDied { killer: damage.attacker }, target);
           }
       }
   }
   ```

3. **Triggering Combat Events**:
   ```rust
   // Directly target the damaged entity
   commands.trigger_targets(
       DamageDealt { attacker: caster_entity, amount: 45.0, is_crit: true },
       target_entity,
   );
   ```

---

### Phase 3: Reactive UI & HUD Elements (`bevymmo_presentation`)

#### Current Problem
Cast bar UI ([`crates/presentation/src/spells/cast_bar.rs`](file:///C:/Users/alexb/Documents/Coding/Rust/BevyMMO/crates/presentation/src/spells/cast_bar.rs#L210)) maintains a local resource `ObservedCasts` and runs per-frame systems to sync screen-space UI nodes with spell progress.

#### Observer-Driven Solution
1. **Cast Bar Spawning & Toggling**:
   ```rust
   pub fn on_spell_cast_start(
       trigger: On<Add, ActiveSpellCast>,
       casts: Query<&ActiveSpellCast>,
       mut commands: Commands,
   ) {
       let entity = trigger.entity();
       let cast = casts.get(entity).unwrap();

       // Spawn screen-space or world-space UI cast bar directly linked to caster entity
       commands.spawn((
           CastBarUI { caster: entity },
           // Node layout components...
       ));
   }

   pub fn on_spell_cast_end(
       trigger: On<Remove, ActiveSpellCast>,
       ui_bars: Query<(Entity, &CastBarUI)>,
       mut commands: Commands,
   ) {
       let caster = trigger.entity();
       for (ui_entity, bar) in ui_bars.iter() {
           if bar.caster == caster {
               commands.entity(ui_entity).despawn_recursive();
           }
       }
   }
   ```

2. **Floating Combat Text**:
   ```rust
   pub fn spawn_floating_combat_text(
       trigger: On<DamageDealt>,
       transforms: Query<&Transform>,
       mut commands: Commands,
   ) {
       let target = trigger.entity();
       let damage = trigger.event();

       if let Ok(transform) = transforms.get(target) {
           // Spawn floating 3D text at target location
       }
   }
   ```

---

### Phase 4: Structural Invariants via Component Hooks (`bevymmo_shared` / `bevymmo_server`)

Use **Component Hooks** for low-level structural guarantees where adding Component A must automatically manage Component B without waiting for a system execution.

#### 1. Automatic Component Attachment
```rust
#[derive(Component)]
#[component(on_add = attach_spatial_listener)]
pub struct LocalPlayerMarker;

fn attach_spatial_listener(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    // Automatically attach audio spatial listener when LocalPlayerMarker is inserted
    world.commands().entity(entity).insert(SpatialListener::default());
}
```

#### 2. Health Bounds & Clamping
```rust
#[derive(Component)]
#[component(on_replace = clamp_health_values)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

fn clamp_health_values(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    if let Some(mut health) = world.get_mut::<Health>(entity) {
        health.current = health.current.clamp(0.0, health.max);
    }
}
```

---

## 5. Implementation & Migration Steps

1. **Audit Existing Systems**: Identify polling queries in `bevymmo_presentation` and `bevymmo_server` that search for added/removed components using `Added<T>`, `Changed<T>`, or `Without<T>`.
2. **Refactor Client Visual Spawning**: Replace polling loops for `Predicted`/`Interpolated` visual instantiation with `On<Add, T>` observers in `bevymmo_presentation`.
3. **Refactor Combat Interactions**: Introduce `Trigger<DamageDealt>` and `Trigger<SpellCast>` events in `bevymmo_shared` and register corresponding server observers.
4. **Implement Component Hooks**: Add `on_add` / `on_remove` hooks to core data components (`Health`, `LocalPlayerMarker`) for invariant enforcement.
5. **Verify Protocol & Network Gating**: Ensure observers comply with BevyMMO run conditions (`has_server` and `has_client`), preventing client visual observers from firing on headless servers.

---

## 6. Guidelines & Best Practices

- **Avoid Observer Recursion**: Do not trigger an event inside an observer handler that causes the same observer to fire endlessly without exit conditions.
- **Respect Application Roles**: Gate observer registrations with `.run_if(has_client)` or `.run_if(has_server)` in plugins.
- **Keep Shared Pure**: Define `Event` payloads in `bevymmo_shared`, but register execution logic observers in `bevymmo_server` or `bevymmo_presentation`.
