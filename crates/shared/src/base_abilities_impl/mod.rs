//! Concrete `BaseAbility` implementations — un file per gesto, mirror di
//! `crate::spells_impl`/`crate::items_impl`.

pub mod arcane_gale;
pub mod arcane_orb;
pub mod arcane_seal;
pub mod binding_seal;
pub mod meteor_strike;

use bevy::prelude::ResMut;

use crate::abilities::BaseAbilityRegistry;

/// Registra ogni gesto base disponibile. Chiamato una volta a Startup, sia
/// client sia server (stesso pattern di `register_default_items`).
pub fn register_default_base_abilities(mut registry: ResMut<BaseAbilityRegistry>) {
    arcane_orb::ArcaneOrb::register(&mut registry);
    arcane_seal::ArcaneSeal::register(&mut registry);
    binding_seal::BindingSeal::register(&mut registry);
    arcane_gale::ArcaneGale::register(&mut registry);
    meteor_strike::MeteorStrike::register(&mut registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[test]
    fn register_default_base_abilities_populates_the_registry() {
        let mut app = App::new();
        app.init_resource::<BaseAbilityRegistry>();
        app.add_systems(Update, register_default_base_abilities);
        app.update();

        let registry = app.world().resource::<BaseAbilityRegistry>();
        assert_eq!(registry.len(), 5);
    }
}
