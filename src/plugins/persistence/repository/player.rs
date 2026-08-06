//! Repository per la tabella `players`.
//!
//! Tutti i metodi sono `async`. Il sito di chiamata previsto è un task Tokio
//! dedicato (o un task `AsyncComputeTaskPool` di Bevy) che possiede una
//! [`PlayerRepository`] e comunica con il game loop tramite canali; **non**
//! attenderli da dentro un sistema Bevy sincrono.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::plugins::persistence::entity::player::{ActiveModel, Column, Entity, PlayerRecord};
use crate::plugins::persistence::entity::player_spell::{
    ActiveModel as SpellActiveModel, Column as SpellColumn, Entity as SpellEntity,
};
use crate::plugins::persistence::entity::player_stats::{
    ActiveModel as StatsActiveModel, Column as StatsColumn, Entity as StatsEntity,
};
use crate::plugins::persistence::error::{PersistenceError, PersistenceResult};
use crate::plugins::persistence::normalize_name;
use crate::plugins::spells::{default_player_spellbook, SpellId, Spellbook};
use crate::stats::components::{CombatStats, MovementStats, StatsBundleData, VitalStats};
use crate::stats::defaults::player_defaults;
use uuid::Uuid;

/// Snapshot completo di un player persistito: record base, statistiche e spellbook.
#[derive(Clone, Debug)]
pub struct PersistedPlayerSnapshot {
    pub player: PlayerRecord,
    pub stats: StatsBundleData,
    pub spellbook: Spellbook,
}

/// Facade CRUD async sopra la tabella `players`.
#[derive(Clone)]
pub struct PlayerRepository {
    db: DatabaseConnection,
}

impl PlayerRepository {
    /// Wrappa una connessione SeaORM esistente. La connessione è clonabile a
    /// basso costo (internamente con `Arc`) quindi il repository può essere
    /// clonato a basso costo per task.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Restituisce il player per `normalized_name`, inserendo una nuova riga
    /// quando è assente. `display_name` è usato solo in fase di creazione; le
    /// righe esistenti mantengono il display name memorizzato.
    ///
    /// Nota sulla concorrenza: si affida al vincolo `UNIQUE` su
    /// `normalized_name`. Se due chiamanti gareggiano per creare la stessa
    /// chiave, lo sconfitto riceverà un [`PersistenceError::Db`] dovuto alla
    /// violazione del vincolo unique; il chiamante può ripetere re-emanando la
    /// chiamata, che a quel punto troverà la riga esistente.
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

    /// Persiste l'ultima posizione nota per `id`.
    ///
    /// Restituisce [`PersistenceError::NotFound`] quando `id` non referenzia un
    /// player memorizzato, così i chiamanti possono distinguere "nessun player"
    /// da genuini fallimenti DB.
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
        let spellbook = self.load_or_create_default_spellbook(player.id).await?;

        Ok(PersistedPlayerSnapshot {
            player,
            stats,
            spellbook,
        })
    }

    pub async fn save_snapshot(
        &self,
        id: Uuid,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
        stats: &StatsBundleData,
        spellbook: &Spellbook,
    ) -> PersistenceResult<()> {
        self.save_position(id, pos_x, pos_y, pos_z).await?;
        self.save_stats(id, stats).await?;
        self.save_spellbook(id, spellbook).await
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

    /// Carica le statistiche per un player dal database.
    ///
    /// Restituisce [`PersistenceError::NotFound`] quando non esiste una riga
    /// stats per il player_id dato.
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

    /// Carica lo spellbook persistito per un player.
    ///
    /// L'ordinamento per `slot_index` mantiene stabile la hotbar dopo il
    /// reconnect, invece di affidarsi all'ordine non deterministico delle righe.
    ///
    /// # Example
    /// ```rust,no_run
    /// # async fn example(repository: &PlayerRepository, player_id: uuid::Uuid) -> crate::plugins::persistence::error::PersistenceResult<()> {
    /// let spellbook = repository.load_spellbook(player_id).await?;
    /// assert!(!spellbook.spells.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load_spellbook(&self, player_id: Uuid) -> PersistenceResult<Spellbook> {
        let spell_rows = SpellEntity::find()
            .filter(SpellColumn::PlayerId.eq(player_id))
            .order_by_asc(SpellColumn::SlotIndex)
            .all(&self.db)
            .await?;

        Ok(Spellbook::from_ids(
            spell_rows
                .into_iter()
                .map(|spell_row| SpellId::new(spell_row.spell_id)),
        ))
    }

    /// Replaces the persisted spellbook for a player.
    ///
    /// A delete-then-insert strategy keeps the table aligned with the ECS
    /// component even when spells are removed or reordered. This method performs
    /// database writes and should run on the persistence runtime, not inside a
    /// synchronous Bevy system.
    ///
    /// # Example
    /// ```rust,no_run
    /// # async fn example(repository: &PlayerRepository, player_id: uuid::Uuid, spellbook: &crate::plugins::spells::Spellbook) -> crate::plugins::persistence::error::PersistenceResult<()> {
    /// repository.save_spellbook(player_id, spellbook).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn save_spellbook(
        &self,
        player_id: Uuid,
        spellbook: &Spellbook,
    ) -> PersistenceResult<()> {
        SpellEntity::delete_many()
            .filter(SpellColumn::PlayerId.eq(player_id))
            .exec(&self.db)
            .await?;

        if spellbook.spells.is_empty() {
            return Ok(());
        }

        let spell_rows = spellbook
            .spells
            .iter()
            .enumerate()
            .map(|(slot_index, spell_id)| SpellActiveModel {
                player_id: Set(player_id),
                spell_id: Set(spell_id.as_str().to_string()),
                slot_index: Set(slot_index as i32),
            });

        SpellEntity::insert_many(spell_rows).exec(&self.db).await?;
        Ok(())
    }

    async fn load_or_create_default_spellbook(
        &self,
        player_id: Uuid,
    ) -> PersistenceResult<Spellbook> {
        let spellbook = self.load_spellbook(player_id).await?;
        if !spellbook.spells.is_empty() {
            return Ok(spellbook);
        }

        let default_spellbook = default_player_spellbook();
        self.save_spellbook(player_id, &default_spellbook).await?;
        Ok(default_spellbook)
    }

    /// Salva o aggiorna le statistiche per un player.
    ///
    /// Se esiste già una riga per il player_id, vengono aggiornati i valori;
    /// altrimenti viene inserita una nuova riga.
    pub async fn save_stats(
        &self,
        player_id: Uuid,
        stats: &StatsBundleData,
    ) -> PersistenceResult<()> {
        // Prima prova l'update
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

        // Se l'update non ha righe interessate, fai un insert
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
