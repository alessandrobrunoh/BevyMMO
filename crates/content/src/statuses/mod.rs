//! Static status content and its registry.

pub mod burn;
pub mod root;
pub mod slow;
pub mod stun;
pub mod swift;

use crate::effects::StatusRegistry;

pub fn default_statuses() -> StatusRegistry {
    let mut registry = StatusRegistry::default();
    burn::register(&mut registry);
    root::register(&mut registry);
    slow::register(&mut registry);
    stun::register(&mut registry);
    swift::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_statuses_contains_all_statuses() {
        assert_eq!(default_statuses().len(), 5);
        assert!(default_statuses().contains(&burn::Burn::status_id()));
        assert!(default_statuses().contains(&root::Root::status_id()));
        assert!(default_statuses().contains(&slow::Slow::status_id()));
        assert!(default_statuses().contains(&stun::Stun::status_id()));
        assert!(default_statuses().contains(&swift::Swift::status_id()));
    }
}
