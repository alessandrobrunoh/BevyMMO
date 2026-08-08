//! Repository for the `prop_overrides` table.
//!
//! All methods are `async` and must be driven on the dedicated
//! [`PersistenceRuntime`], never from within Bevy's render/fixed-update
//! schedules.
//!
//! [`PersistenceRuntime`]: crate::persistence::plugin::PersistenceRuntime

use sea_orm::prelude::{DateTimeUtc, Json};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::persistence::entity::prop_override::{ActiveModel, Column, Entity, Model};

/// Async CRUD facade over the `prop_overrides` table.
#[derive(Clone)]
pub struct PropOverrideRepository {
    db: DatabaseConnection,
}

impl PropOverrideRepository {
    /// Wraps an existing SeaORM connection. Cheap to clone (internally `Arc`).
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Returns all overrides recorded for `map_id`, used by the server to
    /// merge overrides on top of the static map manifest at load time.
    pub async fn list_for_map(&self, map_id: &str) -> Result<Vec<Model>, sea_orm::DbErr> {
        Entity::find()
            .filter(Column::MapId.eq(map_id))
            .all(&self.db)
            .await
    }

    /// Insert-or-update a single override keyed by `(map_id, prop_id)`.
    ///
    /// Passing `None` for `transform_json` / `tint` / `removed_at` clears that
    /// field in the stored row; callers wanting to leave a field untouched on
    /// update should read the current row first and re-supply its value.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert(
        &self,
        map_id: &str,
        prop_id: &str,
        transform_json: Option<String>,
        tint: Option<Json>,
        removed_at: Option<DateTimeUtc>,
    ) -> Result<(), sea_orm::DbErr> {
        let update_result = Entity::update_many()
            .col_expr(Column::TransformJson, transform_json.clone().into())
            .col_expr(Column::Tint, tint.clone().into())
            .col_expr(Column::RemovedAt, removed_at.into())
            .filter(Column::MapId.eq(map_id))
            .filter(Column::PropId.eq(prop_id))
            .exec(&self.db)
            .await?;

        if update_result.rows_affected == 0 {
            let new_row = ActiveModel {
                map_id: Set(map_id.to_string()),
                prop_id: Set(prop_id.to_string()),
                transform_json: Set(transform_json),
                tint: Set(tint),
                removed_at: Set(removed_at),
                updated_at: Set(None),
            };
            new_row.insert(&self.db).await?;
        }

        Ok(())
    }
}
