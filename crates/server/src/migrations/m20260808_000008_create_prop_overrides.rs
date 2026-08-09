use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
enum PropOverridesTable {
    Table,
    MapId,
    PropId,
    TransformJson,
    Tint,
    RemovedAt,
    UpdatedAt,
}

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260808_000008_create_prop_overrides"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Runtime overrides for placed props (GM edits, "tree chopped",
        // "chest looted"). The server merges these on top of the static
        // `MapManifest.props` at load time. Composite PK `(map_id, prop_id)`
        // guarantees one override per prop per map and natural upsert semantics.
        manager
            .create_table(
                Table::create()
                    .table(PropOverridesTable::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PropOverridesTable::MapId).text().not_null())
                    .col(ColumnDef::new(PropOverridesTable::PropId).text().not_null())
                    // JSON-serialized `TransformData`, or NULL when the override
                    // does not touch the transform (e.g. a pure tint or removal).
                    .col(
                        ColumnDef::new(PropOverridesTable::TransformJson)
                            .text()
                            .null(),
                    )
                    // JSON `[f32; 3]` tint, or NULL when unchanged.
                    .col(ColumnDef::new(PropOverridesTable::Tint).json().null())
                    // When the prop was removed at runtime; NULL = not removed.
                    .col(
                        ColumnDef::new(PropOverridesTable::RemovedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PropOverridesTable::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP"),
                    )
                    // Composite primary key (map_id, prop_id)
                    .primary_key(
                        Index::create()
                            .name("pk_prop_overrides")
                            .col(PropOverridesTable::MapId)
                            .col(PropOverridesTable::PropId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PropOverridesTable::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum PropOverrides {
    Table,
    MapId,
    PropId,
    TransformJson,
    Tint,
    RemovedAt,
    UpdatedAt,
}
