//! Risorse per il sistema di targeting.

use bevy::prelude::*;

/// Target correntemente selezionato dal player.
///
/// Contiene l'entity handle del target selezionato, o `None` se nessun target
/// è attivo. La resource è auto-pulita dai sistemi quando il target:
/// - viene despawnato
/// - perde i componenti richiesti (`Position` o `VitalStats`)
/// - muore (`VitalStats::is_dead()`)
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct CurrentTarget {
    /// Entity handle del target selezionato.
    pub entity: Option<Entity>,
}

impl CurrentTarget {
    /// Crea un nuovo `CurrentTarget` con il target specificato.
    pub fn new(entity: Entity) -> Self {
        Self {
            entity: Some(entity),
        }
    }

    /// Crea un nuovo `CurrentTarget` vuoto (nessun target).
    pub fn none() -> Self {
        Self { entity: None }
    }

    /// Restituisce true se c'è un target attivo.
    pub fn is_some(&self) -> bool {
        self.entity.is_some()
    }

    /// Restituisce true se non c'è un target attivo.
    pub fn is_none(&self) -> bool {
        self.entity.is_none()
    }

    /// Pulisce il target corrente.
    pub fn clear(&mut self) {
        self.entity = None;
    }

    /// Imposta il target corrente.
    pub fn set(&mut self, entity: Entity) {
        self.entity = Some(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_target_default_is_none() {
        let target = CurrentTarget::default();
        assert!(target.is_none());
        assert!(!target.is_some());
    }

    #[test]
    fn current_target_none_explicit() {
        let target = CurrentTarget::none();
        assert!(target.is_none());
    }

    #[test]
    fn current_target_with_entity() {
        let entity = Entity::from_bits(42);
        let target = CurrentTarget::new(entity);
        assert!(target.is_some());
        assert_eq!(target.entity, Some(entity));
    }

    #[test]
    fn current_target_clear() {
        let mut target = CurrentTarget::new(Entity::from_bits(100));
        assert!(target.is_some());
        target.clear();
        assert!(target.is_none());
    }

    #[test]
    fn current_target_set() {
        let mut target = CurrentTarget::none();
        assert!(target.is_none());
        target.set(Entity::from_bits(200));
        assert!(target.is_some());
        assert_eq!(target.entity, Some(Entity::from_bits(200)));
    }
}
