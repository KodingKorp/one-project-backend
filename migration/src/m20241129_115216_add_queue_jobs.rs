use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        manager
            .alter_table(
                Table::alter()
                    .table(Jobs::Table)
                    .add_column(ColumnDef::new(Jobs::Queue).string_len(255).not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        manager
            .alter_table(Table::alter().table(Jobs::Table).drop_column(Jobs::Queue).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Jobs {
    Table,
    Queue,
}
