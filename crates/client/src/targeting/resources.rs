//! Targeting system resources.

use bevy::prelude::*;
/// Target currently selected by the player.
///
/// Contains the entity handle of the selected target, or `None` if no target
/// is active. The resource is auto-cleaned by systems when the target:
/// - is despawned
/// - loses required components (`Position` or `VitalStats`)
/// - dies (`VitalStats::is_dead()`)
#[derive(Resource, Default, Debug, Clone, Copy)]
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
        assert!(target.entity.is_none());
    }

    #[test]
    fn current_target_with_entity() {
        let entity = Entity::from_raw_u32(42).expect("valid entity index");
        let target = CurrentTarget::new(entity);
        assert_eq!(target.entity, Some(entity));
    }

    #[test]
    fn current_target_clear() {
        let mut target = CurrentTarget::new(Entity::from_raw_u32(100).expect("valid entity index"));
        assert!(target.entity.is_some());
        target.clear();
        assert!(target.entity.is_none());
    }

    #[test]
    fn current_target_set() {
        let mut target = CurrentTarget::default();
        assert!(target.entity.is_none());
        target.set(Entity::from_raw_u32(200).expect("valid entity index"));
        assert_eq!(
            target.entity,
            Some(Entity::from_raw_u32(200).expect("valid entity index"))
        );
    }
}
