//! Versioned SeaORM migrations, applied by [`PersistencePlugin`] during startup.
//!
//! [`PersistencePlugin`]: crate::plugins::persistence::plugin::PersistencePlugin

use sea_orm_migration::prelude::*;

mod m20260805_000001_create_players;
mod m20260805_000002_create_player_stats;
mod m20260805_000003_create_player_spells;
mod m20260805_000004_rename_fireball_spell_to_ray_of_light;
mod m20260806_000005_rename_followball_spell_to_fireball;
mod m20260806_000006_create_player_hotbar;
mod m20260807_000007_create_player_inventory_and_equipment;
mod m20260808_000008_create_prop_overrides;
mod m20260813_000009_add_equipment_slots;
mod m20260813_000010_create_player_known_glyphs;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260805_000001_create_players::Migration),
            Box::new(m20260805_000002_create_player_stats::Migration),
            Box::new(m20260805_000003_create_player_spells::Migration),
            Box::new(m20260805_000004_rename_fireball_spell_to_ray_of_light::Migration),
            Box::new(m20260806_000005_rename_followball_spell_to_fireball::Migration),
            Box::new(m20260806_000006_create_player_hotbar::Migration),
            Box::new(m20260807_000007_create_player_inventory_and_equipment::Migration),
            Box::new(m20260808_000008_create_prop_overrides::Migration),
            Box::new(m20260813_000009_add_equipment_slots::Migration),
            Box::new(m20260813_000010_create_player_known_glyphs::Migration),
        ]
    }
}
