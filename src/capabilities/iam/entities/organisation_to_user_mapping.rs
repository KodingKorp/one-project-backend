use chrono::NaiveDateTime as DateTime;
use sea_orm::{entity::prelude::*, FromQueryResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "organisation_to_user_mapping")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub org_id: i32,
    pub user_id: i32,
    pub role: Role,            // Required: Define roles like "admin", "member", etc.
    pub status: UserOrgStatus, // Required: active, inactive, invited
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::organisations::Entity",
        from = "Column::OrgId",
        to = "super::organisations::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Organisation,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    User,
}

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(10))")]
pub enum Role {
    #[sea_orm(string_value = "admin")]
    Admin,
    #[sea_orm(string_value = "member")]
    Member,
}

impl TryFrom<String> for Role {
    type Error = sea_orm::DbErr;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "admin" => Ok(Role::Admin),
            "member" => Ok(Role::Member),
            _ => Err(sea_orm::DbErr::Custom(format!("Invalid role: {}", value))),
        }
    }
}

#[derive(EnumIter, DeriveActiveEnum, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(10))")]
pub enum UserOrgStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "inactive")]
    Inactive,
    #[sea_orm(string_value = "invited")]
    Invited,
}

impl TryFrom<String> for UserOrgStatus {
    type Error = sea_orm::DbErr;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "active" => Ok(UserOrgStatus::Active),
            "inactive" => Ok(UserOrgStatus::Inactive),
            "invited" => Ok(UserOrgStatus::Invited),
            _ => Err(sea_orm::DbErr::Custom(format!("Invalid status: {}", value))),
        }
    }
}

impl Related<super::organisations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organisation.def()
    }
}
impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Serialize, Deserialize, FromQueryResult)]
pub struct UserObjectWithMappingModel {
    pub pid: Uuid,
    pub id: i32,
    pub email: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub role: Role,
    pub status: UserOrgStatus,
    pub org_id: i32,
}
