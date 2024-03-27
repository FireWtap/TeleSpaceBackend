use crate::m20240221_185740_files::Files;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .create_table(
                Table::create()
                    .table(Chunks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Chunks::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Chunks::TelegramFileId)
                            .string()
                            .unique_key()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Chunks::Order).integer().not_null())
                    .col(ColumnDef::new(Chunks::File).integer().not_null())
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .name("FK_File_Chunks")
                            .from_tbl(Chunks::Table)
                            .from_col(Chunks::File)
                            .to_tbl(Files::Table)
                            .to_col(Files::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .drop_table(Table::drop().table(Chunks::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Chunks {
    Table,
    Id,
    Order,
    TelegramFileId,
    File,
}
