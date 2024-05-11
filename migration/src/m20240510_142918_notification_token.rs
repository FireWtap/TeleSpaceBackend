use std::ops::Not;

use sea_orm_migration::prelude::*;

use crate::m20240221_184457_users::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .create_table(
                Table::create()
                    .table(NotificationTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NotificationTokens::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(NotificationTokens::User)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NotificationTokens::TokenNotification)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager
            .drop_table(Table::drop().table(NotificationTokens::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum NotificationTokens {
    Table,
    Id,
    User,
    TokenNotification,
}
