use crate::m20240221_184457_users::Users;
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
                    .table(Files::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Files::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Files::Filename).string().not_null())
                    .col(ColumnDef::new(Files::Type).boolean().not_null())
                    .col(ColumnDef::new(Files::OriginalSize).integer().not_null())
                    .col(ColumnDef::new(Files::User).integer().not_null())
                    .col(
                        ColumnDef::new(Files::UploadTime)
                            .timestamp()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(ColumnDef::new(Files::LocallyStored).boolean())
                    .col(ColumnDef::new(Files::LastDownload).timestamp())
                    .col(ColumnDef::new(Files::ParentDir).integer().null())
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .name("FK_User_Files")
                            .from_tbl(Files::Table)
                            .from_col(Files::User)
                            .to_tbl(Users::Table)
                            .to_col(Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from_tbl(Files::Table)
                            .from_col(Files::ParentDir)
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
            .drop_table(Table::drop().table(Files::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Files {
    Table,
    Id,
    Filename,
    Type,
    OriginalSize,
    UploadTime,
    User,
    LocallyStored,
    LastDownload,
    ParentDir, // Aggiunto
}
