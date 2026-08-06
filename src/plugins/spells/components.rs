//! ECS components for spell-related runtime state.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::context::{CastKind, ChannelMovementPolicy};
use super::registry::SpellId;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HotbarSlot {
    Q,
    W,
    E,
}

#[derive(Component, Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SpellHotbar {
    pub q_spell: Option<SpellId>,
    pub w_spell: Option<SpellId>,
    pub e_spell: Option<SpellId>,
}

impl SpellHotbar {
    pub fn spell_for_slot(&self, slot: HotbarSlot) -> Option<&SpellId> {
        match slot {
            HotbarSlot::Q => self.q_spell.as_ref(),
            HotbarSlot::W => self.w_spell.as_ref(),
            HotbarSlot::E => self.e_spell.as_ref(),
        }
    }

    pub fn assign(&mut self, slot: HotbarSlot, spell_id: Option<SpellId>) {
        if let Some(id) = &spell_id {
            if self.q_spell.as_ref() == Some(id) {
                self.q_spell = None;
            }
            if self.w_spell.as_ref() == Some(id) {
                self.w_spell = None;
            }
            if self.e_spell.as_ref() == Some(id) {
                self.e_spell = None;
            }
        }

        match slot {
            HotbarSlot::Q => self.q_spell = spell_id,
            HotbarSlot::W => self.w_spell = spell_id,
            HotbarSlot::E => self.e_spell = spell_id,
        }
    }

    pub fn contains(&self, spell_id: &SpellId) -> bool {
        self.q_spell.as_ref() == Some(spell_id)
            || self.w_spell.as_ref() == Some(spell_id)
            || self.e_spell.as_ref() == Some(spell_id)
    }
}

pub fn default_player_hotbar() -> SpellHotbar {
    SpellHotbar {
        q_spell: Some(SpellId::new("attack")),
        w_spell: Some(SpellId::new("fireball")),
        e_spell: Some(SpellId::new("healing_circle")),
    }
}

/// Cooldown timers for spells that have been cast.
///
/// This component tracks remaining cooldown time for each spell cast by an entity.
#[derive(Component, Debug, Default)]
pub struct SpellCooldowns {
    /// Map of spell ID to remaining cooldown timer.
    pub timers: HashMap<SpellId, Timer>,
}

impl SpellCooldowns {
    /// Get the remaining cooldown time for a spell.
    ///
    /// Returns `None` if the spell is not on cooldown.
    pub fn get_remaining(&self, id: &SpellId) -> Option<f32> {
        self.timers.get(id).map(|timer| timer.elapsed_secs())
    }

    /// Check if a spell is currently on cooldown.
    pub fn is_on_cooldown(&self, id: &SpellId) -> bool {
        self.timers
            .get(id)
            .is_some_and(|timer| !timer.is_finished())
    }

    /// Start a cooldown for a spell.
    pub fn start_cooldown(&mut self, id: SpellId, duration_seconds: f32) {
        self.timers
            .insert(id, Timer::from_seconds(duration_seconds, TimerMode::Once));
    }

    /// Tick all cooldown timers.
    pub fn tick(&mut self, delta: std::time::Duration) {
        for timer in self.timers.values_mut() {
            timer.tick(delta);
        }
    }

    /// Clean up finished cooldown timers to free memory.
    pub fn cleanup_finished(&mut self) {
        self.timers.retain(|_, timer| !timer.is_finished());
    }
}

/// Soglia di spostamento orizzontale (in unità) oltre la quale un cast-time
/// o un channeling `InterruptOnMove` viene cancellato. Tunable.
pub const MOVEMENT_INTERRUPT_EPSILON: f32 = 0.05;

/// Stato server-authoritative di una spell in fase di cast (CastTime) o
/// channeling. Il sistema [`crate::plugins::spells::systems::advance_cast_progress`]
/// lo ticka ogni frame e decide quando lanciare l'effetto.
///
/// Una sola istanza per caster: starting una nuova spell mentre questo
/// componente esiste cancella quella precedente.
#[derive(Component, Debug)]
pub struct CastProgress {
    pub spell_id: SpellId,
    pub kind: CastKind,
    pub elapsed_seconds: f32,
    /// Per `CastTime`: tempo richiesto prima del fire. Per `Channeling` è
    /// ignorato (open-ended).
    pub required_seconds: f32,
    /// Policy di interruzione col movimento, copiata dalla `SpellConfig` al
    /// momento dello spawn per evitare un lookup ad ogni tick.
    pub channel_movement: ChannelMovementPolicy,
    /// Snapshot della posizione del caster all'ultimo tick, per rilevare
    /// spostamenti che debbano interrompere il cast.
    pub last_position: Vec3,
    pub target_position: Option<Vec3>,
    pub target_entity: Option<Entity>,
    /// Comando movimento attivo quando il cast è iniziato. Serve per ignorare
    /// il movimento precedente e interrompere solo su un nuovo click/input.
    pub movement_input_at_start: Option<Vec3>,
    /// Accumulatore per il tick interval del channeling. Quando supera
    /// `tick_interval_seconds` la spell viene ri-eseguita.
    pub channel_tick_accumulator_seconds: f32,
    pub tick_interval_seconds: f32,
}
