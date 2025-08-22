use crate::capabilities::iam::entities;
use crate::capabilities::iam::entities::organisations::OrganisationStatus;
use entities::organisation_to_user_mapping::Entity as OrganisationToUserMappingEntity;
use entities::organisation_to_user_mapping::Model as OrganisationToUserMapping;
use entities::organisation_to_user_mapping::Role;
use entities::organisation_to_user_mapping::{UserObjectWithMappingModel, UserOrgStatus};
use entities::organisations::Entity as OrganisationEntity;
use entities::organisations::Model as Organisation;
use entities::users::Entity as UserEntity;
use entities::users::Model as User;
use sea_orm::prelude::Uuid;
use sea_orm::IntoActiveModel;
use sea_orm::PaginatorTrait;
use sea_orm::QueryOrder;
use sea_orm::QuerySelect;
use sea_orm::RelationTrait;
use sea_orm::{ColumnTrait, Set};
use sea_orm::{DatabaseConnection, EntityTrait, JoinType, QueryFilter}; // Added JoinType

use crate::capabilities::lib::common_error::CommonError;

// find_organisation_by_id
pub async fn find_organisation_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<Organisation>, CommonError> {
    match OrganisationEntity::find()
        .filter(entities::organisations::Column::Id.eq(id))
        .one(db)
        .await
    {
        Ok(organisation) => Ok(organisation),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

// check if organisation name is available
pub async fn is_organisation_name_available(
    db: &DatabaseConnection,
    name: &str,
) -> Result<bool, CommonError> {
    let result = OrganisationEntity::find()
        .filter(entities::organisations::Column::Name.eq(name))
        .one(db)
        .await;
    match result {
        Ok(organisation) => Ok(organisation.is_none()),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

// create new organisation
pub async fn create_organisation(
    db: &DatabaseConnection,
    name: Option<String>,
) -> Result<Option<Organisation>, CommonError> {
    let organisation = entities::organisations::ActiveModel {
        name: Set(name),
        pid: Set(Uuid::new_v4()),
        status: Set(OrganisationStatus::Active),
        ..Default::default()
    };
    let res = OrganisationEntity::insert(organisation)
        .exec_with_returning(db)
        .await;
    match res {
        Ok(organisation) => Ok(Some(organisation)),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

// update organisation name
pub async fn update_organisation_name(
    db: &DatabaseConnection,
    id: i32,
    name: String,
) -> Result<Organisation, CommonError> {
    let organisation = OrganisationEntity::find()
        .filter(entities::organisations::Column::Id.eq(id))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    if let Some(organisation) = organisation {
        let mut org_active_model = organisation.into_active_model();
        org_active_model.name = Set(Some(name));
        let res = OrganisationEntity::update(org_active_model).exec(db).await;
        match res {
            Ok(organisation) => Ok(organisation),
            Err(e) => Err(CommonError::from(e.to_string())),
        }
    } else {
        Err(CommonError::from("Organisation not found".to_string()))
    }
}

// update organisation status
pub async fn update_organisation_status(
    db: &DatabaseConnection,
    id: i32,
    status: entities::organisations::OrganisationStatus,
) -> Result<Option<Organisation>, CommonError> {
    let organisation = OrganisationEntity::find()
        .filter(entities::organisations::Column::Id.eq(id))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    if let Some(organisation) = organisation {
        let mut organisation = organisation.into_active_model();
        organisation.status = Set(status);
        let res = OrganisationEntity::update(organisation).exec(db).await;
        match res {
            Ok(organisation) => Ok(Some(organisation)),
            Err(e) => Err(CommonError::from(e.to_string())),
        }
    } else {
        Err(CommonError::from("Organisation not found".to_string()))
    }
}

// invite user to organisation
pub async fn invite_user_to_organisation(
    db: &DatabaseConnection,
    org_id: i32,
    user_id: i32,
    role: Role,
) -> Result<OrganisationToUserMapping, CommonError> {
    let organisation = OrganisationEntity::find()
        .filter(entities::organisations::Column::Id.eq(org_id))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    if organisation.is_none() {
        return Err(CommonError::from("Organisation not found".to_string()));
    }
    let user = UserEntity::find()
        .filter(entities::users::Column::Id.eq(user_id))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    if user.is_none() {
        return Err(CommonError::from("User not found".to_string()));
    }
    let mapping = entities::organisation_to_user_mapping::ActiveModel {
        org_id: Set(org_id),
        user_id: Set(user_id),
        status: Set(UserOrgStatus::Invited),
        role: Set(role),
        ..Default::default()
    };
    let res = OrganisationToUserMappingEntity::insert(mapping)
        .exec_with_returning(db)
        .await;
    match res {
        Ok(mapping) => Ok(mapping),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

// get all users in organisation
pub async fn get_all_users_in_organisation(
    db: &DatabaseConnection,
    org_id: i32,
) -> Result<Vec<User>, CommonError> {
    let result = UserEntity::find()
        .filter(entities::organisation_to_user_mapping::Column::OrgId.eq(org_id))
        .join(
            JoinType::InnerJoin,
            entities::users::Relation::OrganisationToUserMapping.def(),
        )
        .all(db)
        .await;
    match result {
        Ok(users) => Ok(users),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

// get all organisations for a user with a specific status
pub async fn get_all_organisations_for_user_with_status(
    db: &DatabaseConnection,
    user_id: i32,
    status: UserOrgStatus,
) -> Result<Vec<Organisation>, CommonError> {
    let result = OrganisationEntity::find() // Find Organisations
        .join(
            // Join with the mapping table
            JoinType::InnerJoin,
            entities::organisations::Relation::OrganisationToUserMapping.def(),
        )
        .filter(entities::organisation_to_user_mapping::Column::UserId.eq(user_id)) // Filter on mapping table columns
        .filter(entities::organisation_to_user_mapping::Column::Status.eq(status))
        .all(db) // Fetch all matching Organisations
        .await;
    match result {
        Ok(organisations) => Ok(organisations),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

// update user status in organisation
pub async fn update_user_status_in_organisation(
    db: &DatabaseConnection,
    org_id: i32,
    user_id: i32,
    status: UserOrgStatus,
) -> Result<Option<OrganisationToUserMapping>, CommonError> {
    let mapping = OrganisationToUserMappingEntity::find()
        .filter(entities::organisation_to_user_mapping::Column::OrgId.eq(org_id))
        .filter(entities::organisation_to_user_mapping::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    if let Some(mapping) = mapping {
        let mut mapping = mapping.into_active_model();
        mapping.status = Set(status);
        let res = OrganisationToUserMappingEntity::update(mapping)
            .exec(db)
            .await;
        match res {
            Ok(mapping) => Ok(Some(mapping)),
            Err(e) => Err(CommonError::from(e.to_string())),
        }
    } else {
        Err(CommonError::from("Mapping not found".to_string()))
    }
}

// update user role in organisation while making sure atleast one admin exists
pub async fn update_user_role_in_organisation(
    db: &DatabaseConnection,
    org_id: i32,
    user_id: i32,
    role: Role,
) -> Result<Option<OrganisationToUserMapping>, CommonError> {
    let mapping = OrganisationToUserMappingEntity::find()
        .filter(entities::organisation_to_user_mapping::Column::OrgId.eq(org_id))
        .filter(entities::organisation_to_user_mapping::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    if let Some(mapping) = mapping {
        let mut mapping = mapping.into_active_model();
        mapping.role = Set(role);
        let res = OrganisationToUserMappingEntity::update(mapping)
            .exec(db)
            .await;
        match res {
            Ok(mapping) => Ok(Some(mapping)),
            Err(e) => Err(CommonError::from(e.to_string())),
        }
    } else {
        Err(CommonError::from("Mapping not found".to_string()))
    }
}

// remove user from organisation (update status to inactive)
pub async fn remove_user_from_organisation(
    db: &DatabaseConnection,
    org_id: i32,
    user_id: i32,
) -> Result<Option<OrganisationToUserMapping>, CommonError> {
    let mapping = OrganisationToUserMappingEntity::find()
        .filter(entities::organisation_to_user_mapping::Column::OrgId.eq(org_id))
        .filter(entities::organisation_to_user_mapping::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    if let Some(mapping) = mapping {
        let mut mapping = mapping.into_active_model();
        mapping.status = Set(UserOrgStatus::Inactive);
        let res = OrganisationToUserMappingEntity::update(mapping)
            .exec(db)
            .await;
        match res {
            Ok(mapping) => Ok(Some(mapping)),
            Err(e) => Err(CommonError::from(e.to_string())),
        }
    } else {
        Err(CommonError::from("Mapping not found".to_string()))
    }
}

// active user in organisation
pub async fn activate_user_in_organisation(
    db: &DatabaseConnection,
    org_id: i32,
    user_id: i32,
) -> Result<Option<OrganisationToUserMapping>, CommonError> {
    let mapping = OrganisationToUserMappingEntity::find()
        .filter(entities::organisation_to_user_mapping::Column::OrgId.eq(org_id))
        .filter(entities::organisation_to_user_mapping::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    if let Some(mapping) = mapping {
        let mut mapping = mapping.into_active_model();
        mapping.status = Set(UserOrgStatus::Active);
        let res = OrganisationToUserMappingEntity::update(mapping)
            .exec(db)
            .await;
        match res {
            Ok(mapping) => Ok(Some(mapping)),
            Err(e) => Err(CommonError::from(e.to_string())),
        }
    } else {
        Err(CommonError::from("Mapping not found".to_string()))
    }
}

// get user's most recent orgnaisation
pub async fn get_users_most_recent_organisation(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<Option<Organisation>, CommonError> {
    let result = OrganisationEntity::find()
        .join(
            JoinType::InnerJoin,
            entities::organisations::Relation::OrganisationToUserMapping.def(),
        )
        .filter(
            entities::organisations::Column::Status
                .eq(entities::organisations::OrganisationStatus::Active),
        )
        .filter(entities::organisation_to_user_mapping::Column::Status.eq(UserOrgStatus::Active))
        .filter(entities::organisation_to_user_mapping::Column::UserId.eq(user_id))
        .order_by(
            entities::organisation_to_user_mapping::Column::CreatedAt,
            sea_orm::Order::Desc,
        )
        .one(db)
        .await;
    match result {
        Ok(organisation) => Ok(organisation),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

// add user to organisation with role and status
pub async fn add_user_to_organisation(
    db: &DatabaseConnection,
    org_id: i32,
    user_id: i32,
    role: Role,
    status: UserOrgStatus,
) -> Result<Option<OrganisationToUserMapping>, CommonError> {
    let mapping = entities::organisation_to_user_mapping::ActiveModel {
        org_id: Set(org_id),
        user_id: Set(user_id),
        role: Set(role),
        status: Set(status),
        ..Default::default()
    };
    let res = OrganisationToUserMappingEntity::insert(mapping)
        .exec_with_returning(db)
        .await;
    match res {
        Ok(mapping) => Ok(Some(mapping)),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

// get user mapping for the organisation
pub async fn get_user_mapping_for_organisation(
    db: &DatabaseConnection,
    org_id: i32,
    user_id: i32,
) -> Result<Option<OrganisationToUserMapping>, CommonError> {
    let mapping = OrganisationToUserMappingEntity::find()
        .filter(entities::organisation_to_user_mapping::Column::OrgId.eq(org_id))
        .filter(entities::organisation_to_user_mapping::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    match mapping {
        Some(mapping) => Ok(Some(mapping)),
        None => Err(CommonError::from("Mapping not found".to_string())),
    }
}

// get mapping by id
pub async fn get_mapping_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<OrganisationToUserMapping>, CommonError> {
    let mapping = OrganisationToUserMappingEntity::find()
        .filter(entities::organisation_to_user_mapping::Column::Id.eq(id))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    match mapping {
        Some(mapping) => Ok(Some(mapping)),
        None => Err(CommonError::from("Mapping not found".to_string())),
    }
}

pub async fn get_users_in_organisation_by_status(
    db: &DatabaseConnection,
    org_id: i32,
    status: UserOrgStatus,
    mut page: u64,
    mut page_size: u64,
) -> Result<Vec<UserObjectWithMappingModel>, CommonError> {
    if page == 0 {
        page = 1;
    }
    if page_size == 0 {
        page_size = 10;
    }
    let result: Result<Vec<UserObjectWithMappingModel>, sea_orm::DbErr> = UserEntity::find()
        .select_only()
        .column(entities::users::Column::Id)
        .column(entities::users::Column::Email)
        .column(entities::users::Column::Pid)
        .column(entities::organisation_to_user_mapping::Column::Role)
        .column(entities::organisation_to_user_mapping::Column::Status)
        .column(entities::organisation_to_user_mapping::Column::OrgId)
        .column(entities::organisation_to_user_mapping::Column::CreatedAt)
        .column(entities::organisation_to_user_mapping::Column::UpdatedAt)
        .filter(entities::organisation_to_user_mapping::Column::OrgId.eq(org_id))
        .filter(entities::organisation_to_user_mapping::Column::Status.eq(status))
        .join(
            JoinType::InnerJoin,
            entities::users::Relation::OrganisationToUserMapping.def(),
        )
        .order_by(
            entities::organisation_to_user_mapping::Column::CreatedAt,
            sea_orm::Order::Desc,
        )
        .into_model()
        .paginate(db, page_size)
        .fetch_page(page - 1)
        .await;
    match result {
        Ok(users) => Ok(users),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

// get all users' active organisations
pub async fn get_all_users_active_organisations(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<Vec<Organisation>, CommonError> {
    let result = OrganisationEntity::find()
        .join(
            JoinType::InnerJoin,
            entities::organisations::Relation::OrganisationToUserMapping.def(),
        )
        .filter(entities::organisation_to_user_mapping::Column::UserId.eq(user_id))
        .filter(entities::organisation_to_user_mapping::Column::Status.eq(UserOrgStatus::Active))
        .filter(
            entities::organisations::Column::Status
                .eq(entities::organisations::OrganisationStatus::Active),
        )
        .all(db)
        .await;
    match result {
        Ok(organisations) => Ok(organisations),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

/// deactivate organisation
pub async fn deactivate_organisation(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<Organisation>, CommonError> {
    let organisation = OrganisationEntity::find()
        .filter(entities::organisations::Column::Id.eq(id))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    if let Some(organisation) = organisation {
        let mut organisation = organisation.into_active_model();
        organisation.status = Set(entities::organisations::OrganisationStatus::Inactive);
        let res = OrganisationEntity::update(organisation).exec(db).await;
        match res {
            Ok(organisation) => Ok(Some(organisation)),
            Err(e) => Err(CommonError::from(e.to_string())),
        }
    } else {
        Err(CommonError::from("Organisation not found".to_string()))
    }
}
