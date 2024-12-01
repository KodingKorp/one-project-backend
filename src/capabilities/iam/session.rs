use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::capabilities::lib::common_error::CommonError;

use super::entities::session;

pub async fn create_session(
    db: &DatabaseConnection,
    user_id: i32,
    user_agent: String,
    ip: String,
) -> Result<session::Model, CommonError> {
    let session = session::ActiveModel {
        user_id: Set(user_id),
        user_agent: Set(user_agent),
        ip: Set(ip),
        pid: Set(Uuid::new_v4()),
        ..Default::default()
    };
    match session::Entity::insert(session)
        .exec_with_returning(db)
        .await
    {
        Ok(session) => Ok(session),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

pub async fn find_session_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<session::Model>, CommonError> {
    match session::Entity::find()
        .filter(session::Column::Id.eq(id))
        .one(db)
        .await
    {
        Ok(session) => Ok(session),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

pub async fn find_session_by_pid(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<session::Model>, CommonError> {
    match session::Entity::find()
        .filter(session::Column::Pid.eq(id))
        .one(db)
        .await
    {
        Ok(session) => Ok(session),
        Err(e) => Err(CommonError::from(e.to_string())),
    }
}

pub async fn delete_session_by_id(db: &DatabaseConnection, id: i32) -> Result<(), CommonError> {
    let session = find_session_by_id(db, id).await?;
    if let Some(session) = session {
        let session: session::ActiveModel = session.into();
        session
            .delete(db)
            .await
            .map_err(|e| CommonError::from(e.to_string()))?;
        return Ok(());
    }
    Ok(())
}
