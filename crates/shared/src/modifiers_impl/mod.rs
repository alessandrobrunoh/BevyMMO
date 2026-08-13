//! Concrete `Modifier` implementations — un file per Modificatore, mirror di
//! `crate::spells_impl`.

pub mod espandere;

use bevy::prelude::ResMut;

use crate::abilities::ModifierRegistry;

pub fn register_default_modifiers(mut registry: ResMut<ModifierRegistry>) {
    espandere::EspandereModifier::register(&mut registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[test]
    fn register_default_modifiers_populates_the_registry() {
        let mut app = App::new();
        app.init_resource::<ModifierRegistry>();
        app.add_systems(Update, register_default_modifiers);
        app.update();

        let registry = app.world().resource::<ModifierRegistry>();
        assert_eq!(registry.len(), 1);
    }
}
