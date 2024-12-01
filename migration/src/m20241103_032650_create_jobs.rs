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
                .if_not_exists()
                .table(Jobs::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Jobs::Id)
                        .integer()
                        .auto_increment()
                        .primary_key(),
                )
                .col(ColumnDef::new(Jobs::LinkedJobId).integer().null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk-jobs-job_id")
                                .from(Jobs::Table, Jobs::LinkedJobId)
                                .to(Jobs::Table, Jobs::Id),
                        )
                .col(ColumnDef::new(Jobs::JobId).string_len(255).not_null())
                .col(ColumnDef::new(Jobs::JobType).string_len(20).not_null())
                .col(ColumnDef::new(Jobs::Status).string_len(20).not_null())
                .col(ColumnDef::new(Jobs::Retries).integer().not_null().default(0))
                .col(ColumnDef::new(Jobs::MaxRetries).integer().not_null().default(0))
                .col(ColumnDef::new(Jobs::Payload).text().null())
                .col(ColumnDef::new(Jobs::Output).text().null())
                .col(ColumnDef::new(Jobs::Pattern).string_len(255).null())
                .col(ColumnDef::new(Jobs::Delay).integer().not_null().default(0))
                .col(
                    ColumnDef::new(Jobs::LastRanAt)
                        .timestamp()
                        .null(),
                )
                .col(
                    ColumnDef::new(Jobs::NextRunAt)
                        .timestamp()
                        .null(),
                )
                .col(
                    ColumnDef::new(Jobs::CompletedAt)
                        .timestamp()
                        .null(),
                )
                .col(
                    ColumnDef::new(Jobs::FailedAt)
                        .timestamp()
                        .null(),
                )
                .col(
                    ColumnDef::new(Jobs::CreatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Jobs::UpdatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .drop_table(Table::drop().table(Jobs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Jobs {
    Table,
    Id,
    JobId,
    Payload,
    JobType,
    Status,
    Retries,
    MaxRetries,
    NextRunAt,
    CompletedAt,
    FailedAt,
    LinkedJobId,
    LastRanAt,
    Output,
    Pattern,
    Delay,
    CreatedAt,
    UpdatedAt,
}
