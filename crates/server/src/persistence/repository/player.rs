//! Repository for the `players` table.
//!
//! All methods are `async`. The intended call site is a dedicated Tokio task
//! (or Bevy `AsyncComputeTaskPool` task) that owns a
//! [`PlayerRepository`] and communicates with the game loop via channels; **do not**
//! await them from inside a synchronous Bevy system.

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::persistence::entity::player::{ActiveModel, Column, Entity, PlayerRecord};
use crate::persistence::entity::player_equipment::{
    ActiveModel as EquipmentActiveModel, Column as EquipmentColumn, Entity as EquipmentEntity,
};
use crate::persistence::entity::player_hotbar::{
    ActiveModel as HotbarActiveModel, Column as HotbarColumn, Entity as HotbarEntity,
};
use crate::persistence::entity::player_inventory::{
    ActiveModel as InventoryActiveModel, Column as InventoryColumn, Entity as InventoryEntity,
};
use crate::persistence::entity::player_stats::{
    ActiveModel as StatsActiveModel, Column as StatsColumn, Entity as StatsEntity,
};
use crate::persistence::error::{PersistenceError, PersistenceResult};
use crate::persistence::normalize_name;
use bevymmo_shared::items::components::{Equipment, Inventory};
use bevymmo_shared::items::registry::ItemId;
use bevymmo_shared::spells::{default_player_hotbar, SpellHotbar, SpellId};
use bevymmo_shared::stats::components::{CombatStats, MovementStats, StatsBundleData, VitalStats};
use bevymmo_shared::stats::defaults::player_defaults;
use uuid::Uuid;

/// Full snapshot of a persisted player: base record, stats, hotbar,
/// inventory, and equipment.
#[derive(Clone, Debug)]
pub struct PersistedPlayerSnapshot {
    pub player: PlayerRecord,
    pub stats: StatsBundleData,
    pub hotbar: SpellHotbar,
    pub inventory: Inventory,
    pub equipment: Equipment,
}

/// Async CRUD facade over the `players` table.
#[derive(Clone)]
pub struct PlayerRepository {
    db: DatabaseConnection,
}

impl PlayerRepository {
    /// Wraps an existing SeaORM connection. The connection is cheap to clone
    /// (internally via `Arc`) so the repository can be cheaply cloned per task.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Returns the player for `normalized_name`, inserting a new row
    /// when absent. `display_name` is only used during creation;
    /// existing rows retain their stored display name.
    ///
    /// Concurrency note: relies on the `UNIQUE` constraint on
    /// `normalized_name`. If two callers race to create the same
    /// key, the losing caller will receive a [`PersistenceError::Db`] due to
    /// unique constraint violation; the caller can retry by re-issuing
    /// the call, which will then find the existing row.
    pub async fn find_or_create(
        &self,
        normalized_name: &str,
        display_name: &str,
    ) -> PersistenceResult<PlayerRecord> {
        let key = normalize_name(normalized_name);

        if let Some(existing) = Entity::find()
            .filter(Column::NormalizedName.eq(key.clone()))
            .one(&self.db)
            .await?
        {
            return Ok(existing);
        }

        let new_row = ActiveModel {
            id: Set(Uuid::new_v4()),
            normalized_name: Set(key),
            display_name: Set(display_name.to_string()),
            pos_x: Set(0.0),
            pos_y: Set(0.0),
            pos_z: Set(0.0),
        };
        let inserted = new_row.insert(&self.db).await?;
        Ok(inserted)
    }

    /// Persists the last known position for `id`.
    ///
    /// Returns [`PersistenceError::NotFound`] when `id` does not reference a
    /// stored player, allowing callers to distinguish "no player"
    /// from genuine DB failures.
    pub async fn find_or_create_snapshot(
        &self,
        normalized_name: &str,
        display_name: &str,
    ) -> PersistenceResult<PersistedPlayerSnapshot> {
        let player = self.find_or_create(normalized_name, display_name).await?;
        let stats = match self.load_stats(player.id).await {
            Ok(stats) => stats,
            Err(PersistenceError::NotFound(_)) => {
                let defaults = player_defaults();
                self.save_stats(player.id, &defaults).await?;
                defaults
            }
            Err(error) => return Err(error),
        };
        let hotbar = self.load_or_create_default_hotbar(player.id).await?;
        let inventory = self.load_or_create_default_inventory(player.id).await?;
        let equipment = self.load_or_create_default_equipment(player.id).await?;

        Ok(PersistedPlayerSnapshot {
            player,
            stats,
            hotbar,
            inventory,
            equipment,
        })
    }

    pub async fn save_snapshot(
        &self,
        id: Uuid,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
        stats: &StatsBundleData,
        hotbar: &SpellHotbar,
        inventory: &Inventory,
        equipment: &Equipment,
    ) -> PersistenceResult<()> {
        self.save_position(id, pos_x, pos_y, pos_z).await?;
        self.save_stats(id, stats).await?;
        self.save_hotbar(id, hotbar).await?;
        self.save_inventory(id, inventory).await?;
        self.save_equipment(id, equipment).await
    }

    pub async fn save_position(
        &self,
        id: Uuid,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
    ) -> PersistenceResult<()> {
        let result = Entity::update_many()
            .col_expr(Column::PosX, pos_x.into())
            .col_expr(Column::PosY, pos_y.into())
            .col_expr(Column::PosZ, pos_z.into())
            .filter(Column::Id.eq(id))
            .exec(&self.db)
            .await?;

        if result.rows_affected == 0 {
            return Err(PersistenceError::NotFound(format!("player id={id}")));
        }
        Ok(())
    }

    /// Loads stats for a player from the database.
    ///
    /// Returns [`PersistenceError::NotFound`] when no stats row exists for the given player_id.
    pub async fn load_stats(&self, player_id: Uuid) -> PersistenceResult<StatsBundleData> {
        let stats = StatsEntity::find()
            .filter(StatsColumn::PlayerId.eq(player_id))
            .one(&self.db)
            .await?
            .ok_or_else(|| {
                PersistenceError::NotFound(format!("player stats for player_id={player_id}"))
            })?;

        Ok(StatsBundleData {
            movement: MovementStats {
                speed: stats.movement_speed,
            },
            combat: CombatStats {
                attack_power: stats.attack_power,
                armor: stats.armor,
            },
            vital: VitalStats {
                current_health: stats.current_health,
                max_health: stats.max_health,
                max_mana: stats.max_mana,
                mana_regeneration: stats.mana_regeneration,
            },
        })
    }

    /// Loads the persisted hotbar for a player.
    pub async fn load_hotbar(&self, player_id: Uuid) -> PersistenceResult<Option<SpellHotbar>> {
        let hotbar_row = HotbarEntity::find()
            .filter(HotbarColumn::PlayerId.eq(player_id))
            .one(&self.db)
            .await?;

        Ok(hotbar_row.map(|row| SpellHotbar {
            q_spell: row.q_spell.map(SpellId::new),
            w_spell: row.w_spell.map(SpellId::new),
            e_spell: row.e_spell.map(SpellId::new),
        }))
    }

    /// Replaces the persisted hotbar for a player.
    pub async fn save_hotbar(
        &self,
        player_id: Uuid,
        hotbar: &SpellHotbar,
    ) -> PersistenceResult<()> {
        let update_result = HotbarEntity::update_many()
            .col_expr(
                HotbarColumn::QSpell,
                hotbar
                    .q_spell
                    .as_ref()
                    .map(|s| s.as_str().to_string())
                    .into(),
            )
            .col_expr(
                HotbarColumn::WSpell,
                hotbar
                    .w_spell
                    .as_ref()
                    .map(|s| s.as_str().to_string())
                    .into(),
            )
            .col_expr(
                HotbarColumn::ESpell,
                hotbar
                    .e_spell
                    .as_ref()
                    .map(|s| s.as_str().to_string())
                    .into(),
            )
            .filter(HotbarColumn::PlayerId.eq(player_id))
            .exec(&self.db)
            .await?;

        if update_result.rows_affected == 0 {
            let new_hotbar = HotbarActiveModel {
                player_id: Set(player_id),
                q_spell: Set(hotbar.q_spell.as_ref().map(|s| s.as_str().to_string())),
                w_spell: Set(hotbar.w_spell.as_ref().map(|s| s.as_str().to_string())),
                e_spell: Set(hotbar.e_spell.as_ref().map(|s| s.as_str().to_string())),
            };

            new_hotbar.insert(&self.db).await?;
        }

        Ok(())
    }

    async fn load_or_create_default_hotbar(
        &self,
        player_id: Uuid,
    ) -> PersistenceResult<SpellHotbar> {
        if let Some(hotbar) = self.load_hotbar(player_id).await? {
            return Ok(hotbar);
        }

        let default_hotbar = default_player_hotbar();
        self.save_hotbar(player_id, &default_hotbar).await?;
        Ok(default_hotbar)
    }

    /// Loads the persisted inventory for a player.
    ///
    /// Returns `None` when no row exists yet (caller falls back to the default
    /// empty inventory, mirroring `load_hotbar`).
    pub async fn load_inventory(&self, player_id: Uuid) -> PersistenceResult<Option<Inventory>> {
        let Some(row) = InventoryEntity::find()
            .filter(InventoryColumn::PlayerId.eq(player_id))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        let inventory = serde_json::from_str(&row.slots_json).map_err(|e| {
            PersistenceError::Db(sea_orm::DbErr::Custom(format!(
                "failed to parse inventory JSON for player {player_id}: {e}"
            )))
        })?;
        Ok(Some(inventory))
    }

    /// Replaces the persisted inventory for a player (insert-or-update).
    pub async fn save_inventory(
        &self,
        player_id: Uuid,
        inventory: &Inventory,
    ) -> PersistenceResult<()> {
        let slots_json = serde_json::to_string(inventory).map_err(|e| {
            PersistenceError::Db(sea_orm::DbErr::Custom(format!(
                "failed to serialize inventory: {e}"
            )))
        })?;

        let update_result = InventoryEntity::update_many()
            .col_expr(InventoryColumn::SlotsJson, slots_json.clone().into())
            .filter(InventoryColumn::PlayerId.eq(player_id))
            .exec(&self.db)
            .await?;

        if update_result.rows_affected == 0 {
            let new_row = InventoryActiveModel {
                player_id: Set(player_id),
                slots_json: Set(slots_json),
            };
            new_row.insert(&self.db).await?;
        }

        Ok(())
    }

    async fn load_or_create_default_inventory(
        &self,
        player_id: Uuid,
    ) -> PersistenceResult<Inventory> {
        if let Some(inventory) = self.load_inventory(player_id).await? {
            return Ok(inventory);
        }

        let default_inventory = Inventory::default();
        self.save_inventory(player_id, &default_inventory).await?;
        Ok(default_inventory)
    }

    /// Loads the persisted equipment for a player.
    ///
    /// Returns `None` when no row exists yet (caller falls back to the default
    /// empty equipment, mirroring `load_hotbar`).
    pub async fn load_equipment(&self, player_id: Uuid) -> PersistenceResult<Option<Equipment>> {
        let Some(row) = EquipmentEntity::find()
            .filter(EquipmentColumn::PlayerId.eq(player_id))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        Ok(Some(Equipment {
            weapon: row.weapon.map(ItemId::new),
        }))
    }

    /// Replaces the persisted equipment for a player (insert-or-update).
    pub async fn save_equipment(
        &self,
        player_id: Uuid,
        equipment: &Equipment,
    ) -> PersistenceResult<()> {
        let update_result = EquipmentEntity::update_many()
            .col_expr(
                EquipmentColumn::Weapon,
                equipment
                    .weapon
                    .as_ref()
                    .map(|w| w.as_str().to_string())
                    .into(),
            )
            .filter(EquipmentColumn::PlayerId.eq(player_id))
            .exec(&self.db)
            .await?;

        if update_result.rows_affected == 0 {
            let new_row = EquipmentActiveModel {
                player_id: Set(player_id),
                weapon: Set(equipment.weapon.as_ref().map(|w| w.as_str().to_string())),
            };
            new_row.insert(&self.db).await?;
        }

        Ok(())
    }

    async fn load_or_create_default_equipment(
        &self,
        player_id: Uuid,
    ) -> PersistenceResult<Equipment> {
        if let Some(equipment) = self.load_equipment(player_id).await? {
            return Ok(equipment);
        }

        let default_equipment = Equipment::default();
        self.save_equipment(player_id, &default_equipment).await?;
        Ok(default_equipment)
    }

    /// Saves or updates stats for a player.
    ///
    /// If a row already exists for player_id, the values are updated;
    /// otherwise a new row is inserted.
    pub async fn save_stats(
        &self,
        player_id: Uuid,
        stats: &StatsBundleData,
    ) -> PersistenceResult<()> {
        // First try update
        let update_result = StatsEntity::update_many()
            .col_expr(
                StatsColumn::CurrentHealth,
                stats.vital.current_health.into(),
            )
            .col_expr(StatsColumn::MaxHealth, stats.vital.max_health.into())
            .col_expr(StatsColumn::MaxMana, stats.vital.max_mana.into())
            .col_expr(
                StatsColumn::ManaRegeneration,
                stats.vital.mana_regeneration.into(),
            )
            .col_expr(StatsColumn::Armor, stats.combat.armor.into())
            .col_expr(StatsColumn::MovementSpeed, stats.movement.speed.into())
            .col_expr(StatsColumn::AttackPower, stats.combat.attack_power.into())
            .filter(StatsColumn::PlayerId.eq(player_id))
            .exec(&self.db)
            .await?;

        // If update affected no rows, perform insert
        if update_result.rows_affected == 0 {
            let new_stats = StatsActiveModel {
                player_id: Set(player_id),
                current_health: Set(stats.vital.current_health),
                max_health: Set(stats.vital.max_health),
                max_mana: Set(stats.vital.max_mana),
                mana_regeneration: Set(stats.vital.mana_regeneration),
                armor: Set(stats.combat.armor),
                movement_speed: Set(stats.movement.speed),
                attack_power: Set(stats.combat.attack_power),
            };

            new_stats.insert(&self.db).await?;
        }

        Ok(())
    }
}
