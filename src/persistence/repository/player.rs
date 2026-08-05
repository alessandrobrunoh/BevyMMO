//! Repository per la tabella `players`.
//!
//! Tutti i metodi sono `async`. Il sito di chiamata previsto è un task Tokio
//! dedicato (o un task `AsyncComputeTaskPool` di Bevy) che possiede una
//! [`PlayerRepository`] e comunica con il game loop tramite canali; **non**
//! attenderli da dentro un sistema Bevy sincrono.

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::persistence::entity::player::{ActiveModel, Column, Entity, PlayerRecord};
use crate::persistence::error::{PersistenceError, PersistenceResult};
use crate::persistence::normalize_name;
use uuid::Uuid;

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
}
