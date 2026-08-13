//! Concrete `Essence` implementations — un file per Essenza, mirror di
//! `crate::spells_impl`. Aggiungere una nuova Essenza = nuovo file + una
//! riga qui, zero modifiche a codice centrale.

pub mod fuoco;

use bevy::prelude::ResMut;

use crate::abilities::EssenceRegistry;

pub fn register_default_essences(mut registry: ResMut<EssenceRegistry>) {
    fuoco::FuocoEssence::register(&mut registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[test]
    fn register_default_essences_populates_the_registry() {
        let mut app = App::new();
        app.init_resource::<EssenceRegistry>();
        app.add_systems(Update, register_default_essences);
        app.update();

        let registry = app.world().resource::<EssenceRegistry>();
        assert_eq!(registry.len(), 1);
    }
}
