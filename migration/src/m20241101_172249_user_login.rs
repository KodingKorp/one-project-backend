use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_user_table::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .create_table(
                Table::create()
                    .if_not_exists()
                    .table(UserLogin::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserLogin::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserLogin::Strategy).string_len(10).not_null())
                    .col(ColumnDef::new(UserLogin::Value).text().null())
                    .col(ColumnDef::new(UserLogin::UserId).integer().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk-session-user_id")
                                .from(UserLogin::Table, UserLogin::UserId)
                                .to(Users::Table, Users::Id),
                        )
                    .col(
                        ColumnDef::new(UserLogin::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(UserLogin::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(UserLogin::LastUsedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserLogin::Table).to_owned())
            .await
    }
}


#[derive(DeriveIden)]
enum UserLogin {
    Table,
    Id,
    Strategy,
    Value,
    UserId,
    CreatedAt,
    UpdatedAt,
    LastUsedAt
}
