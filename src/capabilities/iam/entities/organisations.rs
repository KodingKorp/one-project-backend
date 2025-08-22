use chrono::NaiveDateTime as DateTime;
use migration::sea_orm;
use sea_orm::{entity::prelude::*, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "organisations")]
pub struct Model {
    pub created_at: DateTime,
    pub updated_at: DateTime,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    #[sea_orm(unique)]
    pub name: Option<String>,
    pub status: OrganisationStatus,
}

// related to organisation_to_user_mapping.rs

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        has_many = "super::organisation_to_user_mapping::Entity",
        from = "Column::Id",
        to = "super::organisation_to_user_mapping::Column::OrgId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    OrganisationToUserMapping,
}

impl Related<super::organisation_to_user_mapping::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrganisationToUserMapping.def()
    }
}

#[derive(
    Default, EnumIter, DeriveActiveEnum, Clone, Debug, Eq, PartialEq, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(10))")]
pub enum OrganisationStatus {
    #[default]
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "inactive")]
    Inactive,
}

impl TryFrom<String> for OrganisationStatus {
    type Error = sea_orm::DbErr;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "active" => Ok(OrganisationStatus::Active),
            "inactive" => Ok(OrganisationStatus::Inactive),
            _ => Err(sea_orm::DbErr::Custom(format!("Invalid status: {}", value))),
        }
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert {
            self.pid = Set(Uuid::new_v4());
        }
        Ok(self)
    }
}
