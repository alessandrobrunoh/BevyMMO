//! Targeting system resources.


use bevy::prelude::*;
/// Target currently selected by the player.
///
/// Contains the entity handle of the selected target, or `None` if no target
/// is active. The resource is auto-cleaned by systems when the target:
/// - is despawned
/// - loses required components (`Position` or `VitalStats`)
/// - dies (`VitalStats::is_dead()`)
#[derive(Resource)]
#[derive(Default, Debug, Clone, Copy)]
pub struct CurrentTarget {
    /// Entity handle of the selected target.
    pub entity: Option<Entity>,
}

impl CurrentTarget {
    /// Creates a new `CurrentTarget` with the specified target.
    pub fn new(entity: Entity) -> Self {
        Self {
            entity: Some(entity),
        }
    }

    /// Creates a new empty `CurrentTarget` (no target).
    pub fn none() -> Self {
        Self { entity: None }
    }

    /// Returns true if there is an active target.
    pub fn is_some(&self) -> bool {
        self.entity.is_some()
    }

    /// Returns true if there is no active target.
    pub fn is_none(&self) -> bool {
        self.entity.is_none()
    }

    /// Clears current target.
    pub fn clear(&mut self) {
        self.entity = None;
    }

    /// Sets current target.
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
        let entity = Entity::from_raw_u32(42).expect("valid entity index");
        let target = CurrentTarget::new(entity);
        assert!(target.is_some());
        assert_eq!(target.entity, Some(entity));
    }

    #[test]
    fn current_target_clear() {
        let mut target = CurrentTarget::new(Entity::from_raw_u32(100).expect("valid entity index"));
        assert!(target.is_some());
        target.clear();
        assert!(target.is_none());
    }

    #[test]
    fn current_target_set() {
        let mut target = CurrentTarget::none();
        assert!(target.is_none());
        target.set(Entity::from_raw_u32(200).expect("valid entity index"));
        assert!(target.is_some());
        assert_eq!(target.entity, Some(Entity::from_raw_u32(200).expect("valid entity index")));
    }
}
