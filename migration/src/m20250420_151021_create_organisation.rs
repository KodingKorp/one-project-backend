use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Organisations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Organisations::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Organisations::Pid)
                            .uuid()
                            .unique_key()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Organisations::Name).string().unique_key()) // Nullable based on Option<String>
                    .col(
                        ColumnDef::new(Organisations::Status)
                            .string() // Based on #[sea_orm(rs_type = "String")]
                            .string_len(10) // Match enum definition if needed
                            .not_null()
                            .default("active"), // Default value
                    )
                    .col(
                        ColumnDef::new(Organisations::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Organisations::UpdatedAt)
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
            .drop_table(Table::drop().table(Organisations::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Organisations {
    Table,
    Id,
    Pid,
    Name,
    Status,
    CreatedAt,
    UpdatedAt,
}
