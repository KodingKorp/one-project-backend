use chrono::NaiveDateTime;
use poem_openapi::Object;
use sea_orm::{ActiveEnum, FromQueryResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::capabilities::lib::common_error::CommonError;

use super::entities::organisation_to_user_mapping::{
    Model as UserOrgMappingModel, Role, UserObjectWithMappingModel, UserOrgStatus,
};
use super::entities::organisations::{Model as OrgModel, OrganisationStatus};
use super::entities::users::Model as UserModel;

#[derive(Object, Serialize, Deserialize, Clone)]
pub struct UserObject {
    pub pid: Uuid,
    pub id: i32,
    pub email: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<UserModel> for UserObject {
    fn from(user: UserModel) -> Self {
        UserObject {
            pid: user.pid,
            id: user.id,
            email: user.email,
            created_at: user.created_at.and_utc().timestamp_millis(),
            updated_at: user.updated_at.and_utc().timestamp_millis(),
        }
    }
}
impl TryFrom<UserObject> for UserModel {
    type Error = CommonError;
    fn try_from(user: UserObject) -> Result<Self, Self::Error> {
        let created_at = match chrono::DateTime::from_timestamp_millis(user.created_at) {
            Some(dt) => dt,
            None => return Err(CommonError::new("Invalid created_at timestamp")),
        };
        let updated_at = match chrono::DateTime::from_timestamp_millis(user.updated_at) {
            Some(dt) => dt,
            None => return Err(CommonError::new("Invalid updated_at timestamp")),
        };
        let created_at: NaiveDateTime = created_at.naive_utc();
        let updated_at: NaiveDateTime = updated_at.naive_utc();
        Ok(UserModel {
            pid: user.pid,
            id: user.id,
            email: user.email,
            created_at,
            updated_at,
        })
    }
}

#[derive(Object, Serialize, Deserialize, FromQueryResult)]
pub struct UserWithMappingObject {
    pub pid: Uuid,
    pub id: i32,
    pub email: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub role: String,
    pub status: String,
    pub org_id: i32,
}
impl TryFrom<UserWithMappingObject> for UserObjectWithMappingModel {
    type Error = CommonError;
    fn try_from(user: UserWithMappingObject) -> Result<Self, Self::Error> {
        let created_at = match chrono::DateTime::from_timestamp_millis(user.created_at) {
            Some(dt) => dt,
            None => return Err(CommonError::new("Invalid created_at timestamp")),
        };
        let updated_at = match chrono::DateTime::from_timestamp_millis(user.updated_at) {
            Some(dt) => dt,
            None => return Err(CommonError::new("Invalid updated_at timestamp")),
        };
        let created_at: NaiveDateTime = created_at.naive_utc();
        let updated_at: NaiveDateTime = updated_at.naive_utc();
        let status =
            UserOrgStatus::try_from(user.status).map_err(|_| CommonError::new("Invalid status"))?;
        let role = Role::try_from(user.role).map_err(|_| CommonError::new("Invalid role"))?;
        Ok(UserObjectWithMappingModel {
            pid: user.pid,
            id: user.id,
            email: user.email,
            created_at,
            updated_at,
            role,
            status,
            org_id: user.org_id,
        })
    }
}
impl From<UserObjectWithMappingModel> for UserWithMappingObject {
    fn from(user: UserObjectWithMappingModel) -> Self {
        UserWithMappingObject {
            pid: user.pid,
            id: user.id,
            email: user.email,
            created_at: user.created_at.and_utc().timestamp_millis(),
            updated_at: user.updated_at.and_utc().timestamp_millis(),
            role: user.role.to_value(),
            status: user.status.to_value(),
            org_id: user.org_id,
        }
    }
}

#[derive(Object, Serialize, Deserialize)]
pub struct SessionObject {
    pub session: Uuid,
    pub user: UserObject,
    pub set_at: i64,
}

#[derive(Object, Serialize, Deserialize)]
pub struct OrganisationObject {
    pub pid: Uuid,
    pub id: i32,
    pub name: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<OrgModel> for OrganisationObject {
    fn from(org: OrgModel) -> Self {
        OrganisationObject {
            pid: org.pid,
            id: org.id,
            name: org.name,
            status: org.status.to_value(),
            created_at: org.created_at.and_utc().timestamp_millis(),
            updated_at: org.updated_at.and_utc().timestamp_millis(),
        }
    }
}

impl TryFrom<OrganisationObject> for OrgModel {
    type Error = CommonError;
    fn try_from(org: OrganisationObject) -> Result<Self, Self::Error> {
        let created_at = match chrono::DateTime::from_timestamp_millis(org.created_at) {
            Some(dt) => dt,
            None => return Err(CommonError::new("Invalid created_at timestamp")),
        };
        let updated_at = match chrono::DateTime::from_timestamp_millis(org.updated_at) {
            Some(dt) => dt,
            None => return Err(CommonError::new("Invalid updated_at timestamp")),
        };
        let status = OrganisationStatus::try_from(org.status)
            .map_err(|_| CommonError::new("Invalid status"))?;
        let created_at: NaiveDateTime = created_at.naive_utc();
        let updated_at: NaiveDateTime = updated_at.naive_utc();
        Ok(OrgModel {
            pid: org.pid,
            id: org.id,
            name: org.name,
            status,
            created_at,
            updated_at,
        })
    }
}

#[derive(Object, Serialize, Deserialize, Clone)]
pub struct UserOrgMappingObject {
    pub id: i32,
    pub user_id: i32,
    pub org_id: i32,
    pub role: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}
impl From<UserOrgMappingModel> for UserOrgMappingObject {
    fn from(mapping: UserOrgMappingModel) -> Self {
        UserOrgMappingObject {
            id: mapping.id,
            user_id: mapping.user_id,
            org_id: mapping.org_id,
            role: mapping.role.to_value(),
            status: mapping.status.to_value(),
            created_at: mapping.created_at.and_utc().timestamp_millis(),
            updated_at: mapping.updated_at.and_utc().timestamp_millis(),
        }
    }
}
impl TryFrom<UserOrgMappingObject> for UserOrgMappingModel {
    type Error = CommonError;
    fn try_from(mapping: UserOrgMappingObject) -> Result<Self, Self::Error> {
        let created_at = match chrono::DateTime::from_timestamp_millis(mapping.created_at) {
            Some(dt) => dt,
            None => return Err(CommonError::new("Invalid created_at timestamp")),
        };
        let updated_at = match chrono::DateTime::from_timestamp_millis(mapping.updated_at) {
            Some(dt) => dt,
            None => return Err(CommonError::new("Invalid updated_at timestamp")),
        };
        let created_at: NaiveDateTime = created_at.naive_utc();
        let updated_at: NaiveDateTime = updated_at.naive_utc();
        let status = UserOrgStatus::try_from(mapping.status)
            .map_err(|_| CommonError::new("Invalid status"))?;
        let role = Role::try_from(mapping.role).map_err(|_| CommonError::new("Invalid role"))?;
        Ok(UserOrgMappingModel {
            id: mapping.id,
            user_id: mapping.user_id,
            org_id: mapping.org_id,
            role,
            status,
            created_at,
            updated_at,
        })
    }
}

#[derive(Object, Serialize, Deserialize)]
pub struct FullSessionInfo {
    pub session: SessionObject,
    pub org: Option<OrganisationObject>,
    pub mapping: Option<UserOrgMappingObject>,
}
impl FullSessionInfo {
    pub fn new(
        mut session: SessionObject,
        org: Option<OrganisationObject>,
        mapping: Option<UserOrgMappingObject>,
    ) -> Self {
        session.set_at = chrono::Utc::now().timestamp_millis();
        FullSessionInfo {
            session,
            org,
            mapping,
        }
    }
    pub fn existing_session(
        session: SessionObject,
        org: Option<OrganisationObject>,
        mapping: Option<UserOrgMappingObject>,
    ) -> Self {
        FullSessionInfo {
            session,
            org,
            mapping,
        }
    }
}
