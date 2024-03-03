use crate::m20240221_185739_files::Files;
use crate::sea_orm::{DeriveActiveEnum, EnumIter};
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
                    .table(TaskList::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaskList::Id)
                            .string()
                            .not_null()
                            .unique_key()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TaskList::File).integer().not_null())
                    .col(ColumnDef::new(TaskList::Status).string().not_null())
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .name("FK_Task_File")
                            .from_tbl(TaskList::Table)
                            .from_col(TaskList::File)
                            .to_tbl(Files::Table)
                            .to_col(Files::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .drop_table(Table::drop().table(TaskList::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum TaskList {
    Table,
    Id,
    File,
    Status,
}
