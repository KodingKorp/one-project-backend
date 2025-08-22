use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::capabilities::lib::common_error::CommonError;

use super::super::entities::user_login::{self, LoginStrategy};

pub async fn add_user_login(
    db: &DatabaseConnection,
    user_id: i32,
    strategy: LoginStrategy,
    strategy_value: Option<String>,
) -> Result<(), CommonError> {
    let user_login = user_login::ActiveModel {
        user_id: Set(user_id),
        strategy: Set(strategy),
        value: Set(strategy_value),
        ..Default::default()
    };
    match user_login::Entity::insert(user_login).exec(db).await {
        Ok(_) => Ok(()),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}
pub async fn find_strategy_value(
    db: &DatabaseConnection,
    user_id: i32,
    strategy: LoginStrategy,
) -> Result<Option<String>, CommonError> {
    let user_login = user_login::Entity::find()
        .filter(user_login::Column::UserId.eq(user_id))
        .filter(user_login::Column::Strategy.eq(strategy.clone()))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    match user_login {
        Some(user_login) => Ok(user_login.value),
        None => Ok(None),
    }
}

pub async fn update_or_create_user_login(
    db: &DatabaseConnection,
    user_id: i32,
    strategy: LoginStrategy,
    strategy_value: Option<String>,
) -> Result<(), CommonError> {
    let user_login = user_login::Entity::find()
        .filter(user_login::Column::UserId.eq(user_id))
        .filter(user_login::Column::Strategy.eq(strategy.clone()))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    if let Some(user_login) = user_login {
        let mut user_login: user_login::ActiveModel = user_login.into();
        user_login.value = Set(strategy_value);
        user_login
            .update(db)
            .await
            .map_err(|e| CommonError::from(e.to_string()))?;
    } else {
        let user_login = user_login::ActiveModel {
            user_id: Set(user_id),
            strategy: Set(strategy),
            value: Set(strategy_value),
            ..Default::default()
        };
        user_login::Entity::insert(user_login)
            .exec_without_returning(db)
            .await
            .map_err(|e| CommonError::from(e.to_string()))?;
    }
    Ok(())
}

pub async fn remove_user_login(
    db: &DatabaseConnection,
    user_id: i32,
    strategy: LoginStrategy,
) -> Result<(), CommonError> {
    let user_login = user_login::Entity::find()
        .filter(user_login::Column::UserId.eq(user_id))
        .filter(user_login::Column::Strategy.eq(strategy.clone()))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    if let Some(user_login) = user_login {
        let user_login: user_login::ActiveModel = user_login.into();
        user_login
            .delete(db)
            .await
            .map_err(|e| CommonError::from(e.to_string()))?;
    }
    Ok(())
}
