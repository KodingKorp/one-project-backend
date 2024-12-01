use crate::capabilities::lib::common_error::CommonError;
use sea_orm::ColumnTrait;
use sea_orm::Set;

use super::entities::users::ActiveModel as ActiveModelUser;
use super::entities::users::Column;
use super::entities::users::Entity as UserEntity;
use super::entities::users::Model as User;
use super::objects::UserObject;
use sea_orm::prelude::Uuid;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter};

pub async fn find_user_by_email(
    db: &DatabaseConnection,
    email: String,
) -> Result<Option<User>, CommonError> {
    let result = UserEntity::find()
        .filter(Column::Email.eq(email))
        .one(db)
        .await;
    match result {
        Ok(user) => Ok(user),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

pub async fn create_user(
    db: &DatabaseConnection,
    email: String,
) -> Result<Option<User>, CommonError> {
    let existing_user_result = find_user_by_email(db, email.clone()).await?;
    if let Some(_) = existing_user_result {
        return Err(CommonError::from(String::from("Already Exists")));
    }
    let user = ActiveModelUser {
        email: Set(email.to_string()),
        pid: Set(Uuid::new_v4()),
        ..Default::default()
    };

    let res = UserEntity::insert(user).exec_with_returning(db).await;
    match res {
        Ok(user) => Ok(Some(user)),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

pub async fn find_user_by_pid(
    db: &DatabaseConnection,
    pid: Uuid,
) -> Result<Option<User>, CommonError> {
    match UserEntity::find().filter(Column::Pid.eq(pid)).one(db).await {
        Ok(user) => Ok(user),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

pub async fn find_user_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<User>, CommonError> {
    match UserEntity::find_by_id(id).one(db).await {
        Ok(user) => Ok(user),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

pub async fn update_user_by_id(
    db: &DatabaseConnection,
    id: i32,
    email: String,
) -> Result<Option<User>, CommonError> {
    let user = ActiveModelUser {
        email: Set(email.to_string()),
        ..Default::default()
    };
    match UserEntity::update(user)
        .filter(Column::Id.eq(id))
        .exec(db)
        .await
    {
        Ok(user) => Ok(Some(user)),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

pub fn convert_model_to_object(user: User) -> UserObject {
    UserObject {
        id: user.id,
        email: user.email,
        pid: user.pid,
        created_at: user.created_at.and_utc().timestamp_millis(),
        updated_at: user.updated_at.and_utc().timestamp_millis(),
    }
}
