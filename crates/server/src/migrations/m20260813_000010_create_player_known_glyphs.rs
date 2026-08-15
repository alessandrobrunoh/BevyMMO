use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260813_000010_create_player_known_glyphs"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PlayerKnownGlyphs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayerKnownGlyphs::PlayerId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    // JSON-serialized `KnownGlyphs` (three sets of glyph ids).
                    .col(ColumnDef::new(PlayerKnownGlyphs::GlyphsJson).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-player_known_glyphs-player_id")
                            .from(PlayerKnownGlyphs::Table, PlayerKnownGlyphs::PlayerId)
                            .to(Players::Table, Players::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PlayerKnownGlyphs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum PlayerKnownGlyphs {
    Table,
    PlayerId,
    GlyphsJson,
}

#[derive(DeriveIden)]
enum Players {
    Table,
    Id,
}
