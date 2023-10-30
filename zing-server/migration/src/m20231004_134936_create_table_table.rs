use sea_orm_migration::prelude::*;

use crate::m20230924_133141_create_user_table::User;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                sea_orm_migration::prelude::Table::create()
                    .table(Table::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Table::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Table::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Table::Game).json_binary())
                    .col(ColumnDef::new(Table::Token).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                sea_orm_migration::prelude::Table::create()
                    .table(TableJoin::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(TableJoin::UserId).integer().not_null())
                    .col(ColumnDef::new(TableJoin::TableId).integer().not_null())
                    .col(ColumnDef::new(TableJoin::TablePos).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-tablejoin-user_id")
                            .from(TableJoin::Table, TableJoin::UserId)
                            .to(User::Table, User::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-tablejoin-table_id")
                            .from(TableJoin::Table, TableJoin::TableId)
                            .to(Table::Table, Table::Id),
                    )
                    .primary_key(
                        Index::create()
                            .col(TableJoin::UserId)
                            .col(TableJoin::TableId),
                    )
                    .to_owned(),
            )
            .await?;

        // table constraint: TablePos must be unique per table
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-table_pos")
                    .table(TableJoin::Table)
                    .col(TableJoin::TableId)
                    .col(TableJoin::TablePos)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                sea_orm_migration::prelude::Table::drop()
                    .table(TableJoin::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                sea_orm_migration::prelude::Table::drop()
                    .table(Table::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
pub enum Table {
    Table,
    Id,
    CreatedAt,
    Token,
    Game,
}

#[derive(DeriveIden)]
enum TableJoin {
    Table,
    UserId,
    TableId,
    TablePos,
}
