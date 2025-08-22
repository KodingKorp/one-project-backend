use sea_orm_migration::prelude::*;

// Import Iden enums from other migrations
use super::m20220101_000001_create_user_table::Users;
use super::m20250420_151021_create_organisation::Organisations;


#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OrganisationToUserMapping::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OrganisationToUserMapping::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(OrganisationToUserMapping::OrgId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganisationToUserMapping::UserId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganisationToUserMapping::Role)
                            .string() // Based on #[sea_orm(rs_type = "String")]
                            .string_len(10)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganisationToUserMapping::Status)
                            .string() // Based on #[sea_orm(rs_type = "String")]
                            .string_len(10)
                            .not_null()
                            .default("active"), // Default value
                    )
                    .col(
                        ColumnDef::new(OrganisationToUserMapping::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(OrganisationToUserMapping::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // Foreign Key to Organisations
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .name("fk-mapping-org_id")
                            .from_tbl(OrganisationToUserMapping::Table)
                            .from_col(OrganisationToUserMapping::OrgId)
                            .to_tbl(Organisations::Table)
                            .to_col(Organisations::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    // Foreign Key to Users
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .name("fk-mapping-user_id")
                            .from_tbl(OrganisationToUserMapping::Table)
                            .from_col(OrganisationToUserMapping::UserId)
                            .to_tbl(Users::Table)
                            .to_col(Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OrganisationToUserMapping::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum OrganisationToUserMapping {
    Table,
    Id,
    OrgId,
    UserId,
    Role,
    Status,
    CreatedAt,
    UpdatedAt,
}
