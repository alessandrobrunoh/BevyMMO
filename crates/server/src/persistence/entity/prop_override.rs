//! SeaORM entity for the `prop_overrides` table.
//!
//! Stores runtime overrides for placed props. The composite primary key
//! `(map_id, prop_id)` matches the natural identity of a `MapManifest` prop.
//!
//! `transform_json` holds a JSON-serialized `TransformData`; `tint` holds a
//! JSON `[f32; 3]`. Both are nullable so a single row can express a partial
//! override (e.g. a tint-only edit). `removed_at` being non-null marks the
//! prop as removed at runtime.

use sea_orm::entity::prelude::*;
use sea_orm::prelude::{DateTimeUtc, Json};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "prop_overrides")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub map_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub prop_id: String,
    /// JSON-serialized `TransformData`, or NULL when the transform is unchanged.
    pub transform_json: Option<String>,
    /// JSON `[f32; 3]` tint, or NULL when unchanged.
    pub tint: Option<Json>,
    /// When the prop was removed at runtime; NULL = not removed.
    pub removed_at: Option<DateTimeUtc>,
    /// Last edit time; defaults to `CURRENT_TIMESTAMP` at the DB level.
    pub updated_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
