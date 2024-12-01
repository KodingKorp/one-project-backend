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
                    .table(Sessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Sessions::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Sessions::Pid).uuid().unique_key().not_null())
                    .col(ColumnDef::new(Sessions::UserAgent).string().null())
                    .col(ColumnDef::new(Sessions::IP).string().null())
                    .col(ColumnDef::new(Sessions::UserId).integer().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk-session-user_id")
                                .from(Sessions::Table, Sessions::UserId)
                                .to(Users::Table, Users::Id),
                        )
                    .col(
                        ColumnDef::new(Sessions::CreatedAt)
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
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Sessions {
    Table,
    Id,
    Pid,
    UserId,
    UserAgent,
    IP,
    CreatedAt
}
