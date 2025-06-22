use sea_orm_migration::prelude::*;

use crate::m20231004_134936_create_table_table::Table;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                sea_orm_migration::prelude::Table::create()
                    .table(GameResults::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GameResults::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GameResults::TableId).integer().not_null())
                    .col(
                        ColumnDef::new(GameResults::CardPoints0)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GameResults::CardPoints1)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GameResults::CardCountPoints0)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GameResults::CardCountPoints1)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GameResults::ZingPoints0)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GameResults::ZingPoints1)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-game_results-table_id")
                            .from(GameResults::Table, GameResults::TableId)
                            .to(Table::Table, Table::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                sea_orm_migration::prelude::Table::drop()
                    .table(GameResults::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum GameResults {
    Table,
    Id,
    TableId,
    CardPoints0,
    CardPoints1,
    CardCountPoints0,
    CardCountPoints1,
    ZingPoints0,
    ZingPoints1,
}
