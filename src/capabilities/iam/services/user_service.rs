use crate::capabilities::iam::constants;
use crate::capabilities::iam::objects::{SessionObject, UserObject};
use crate::capabilities::iam::repositories::users::find_user_by_pid;
use crate::capabilities::lib::common_response;
use crate::capabilities::lib::{common_error::CommonError, common_response::CommonResponse};
use crate::logger;
use poem::session::Session;
use sea_orm::DatabaseConnection;

pub async fn get_current_user(
    db: &DatabaseConnection,
    session: &Session,
) -> Result<CommonResponse<UserObject>, CommonError> {
    let session_data = session.get::<SessionObject>(constants::SESSION_KEY_NAME);
    if session_data.is_none() {
        logger::error("Session data not found");
        return Ok(CommonResponse::Unauthorized);
    }
    let session_object = session_data.unwrap();

    let user = match find_user_by_pid(db, session_object.user.pid).await {
        Ok(user) => user,
        Err(e) => {
            logger::error(&format!("Error finding user: {}", e));
            return Ok(CommonResponse::Unauthorized);
        }
    };
    let user = user.unwrap();
    let user = UserObject::from(user);
    Ok(common_response::ok(user))
}
