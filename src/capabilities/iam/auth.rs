use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::capabilities::background;
use crate::capabilities::crypto::{self, hash_password, verify_password};
use crate::capabilities::lib::common_error::CommonError;

use crate::capabilities::lib::common_response::{self, CommonResponse};
use crate::capabilities::{config, logger, notifications};

use self::notifications::service::NotificationService;

use super::entities::user_login::LoginStrategy;

use super::objects::{SessionObject, UserObject};
use super::session::create_session;
use super::user_login::{find_strategy_value, update_or_create_user_login};
use super::users::{convert_model_to_object, create_user, find_user_by_email};

/// Register a new user
pub async fn register(
    db: &DatabaseConnection,
    email: String,
    strategy: LoginStrategy,
    mut strategy_value: Option<String>,
    orchestrator: background::orchestrator::BackgroundOrchestrator,
) -> Result<CommonResponse<UserObject>, CommonError> {
    let user = find_user_by_email(db, email.clone()).await?;
    if let Some(_) = user {
        logger::info(&format!("User with email {} already exists", email));
        return Ok(CommonResponse::Conflict);
    }
    if strategy == LoginStrategy::Password {
        if let Some(v) = strategy_value {
            let validated = validate_password(&v).unwrap_or(false);
            if !validated {
                return Ok(CommonResponse::BadRequest);
            }
            strategy_value = Some(hash_password(&v)?);
        } else {
            return Ok(CommonResponse::BadRequest);
        }
    }
    let user_result = create_user(db, email.clone()).await?;
    if let Some(user) = user_result {
        update_or_create_user_login(db, user.id, strategy.clone(), strategy_value).await?;
        let user = convert_model_to_object(user);
        if strategy == LoginStrategy::MagicLink {
            let token = create_magic_link_token(&user)?;
            let url = config::get_env::<String>("BASE_LOGIN_LINK") + &token;
            let _ = NotificationService::send_mail(
                "magic_link".to_string(),
                "Magic Link".to_string(),
                user.email.clone(),
                user.email.clone(),
                Some(serde_json::json!({
                    "first_name": user.email.clone(),
                    "url": url
                })),
                orchestrator,
            )
            .await;
            // Send email with token
        }
        return Ok(common_response::ok(user));
    }
    Ok(CommonResponse::InternalServerError)
}
/// Login a user
pub async fn login_password(
    db: &DatabaseConnection,
    email: String,
    password: String,
) -> Result<CommonResponse<UserObject>, CommonError> {
    let validated = validate_password(&password)?;
    if !validated {
        return Err(CommonError::from("Error in password validation"));
    }
    let user = find_user_by_email(db, email.clone()).await?;
    if let Some(user) = user {
        let user_login = find_strategy_value(db, user.id, LoginStrategy::Password).await?;
        if let Some(db_val) = user_login {
            let result = verify_password(&password, &db_val)?;
            if result {
                return Ok(common_response::ok(convert_model_to_object(user)));
            }
        }
    }
    return Ok(CommonResponse::BadRequest);
}

pub async fn login_magic_link(
    db: &DatabaseConnection,
    token: String,
) -> Result<CommonResponse<UserObject>, CommonError> {
    logger::info(&format!("Token: {}", token.clone()));
    let user = verify_magic_link_token(&token)?;
    let user = find_user_by_email(db, user.email.clone()).await?;
    if let Some(user) = user {
        return Ok(common_response::ok(convert_model_to_object(user)));
    }
    return Ok(CommonResponse::BadRequest);
}

pub async fn send_magic_link(
    db: &DatabaseConnection,
    orchestrator: background::orchestrator::BackgroundOrchestrator,
    email: String,
) -> Result<CommonResponse<String>, CommonError> {
    let user = find_user_by_email(db, email.clone()).await?;
    if let Some(user) = user {
        let user = convert_model_to_object(user);
        let token = create_magic_link_token(&user)?;
        let url = config::get_env::<String>("BASE_LOGIN_LINK") + &token;
        let _ = NotificationService::send_mail(
            "magic_link".to_string(),
            "Magic Link".to_string(),
            user.email.clone(),
            user.email.clone(),
            Some(serde_json::json!({
                "first_name": user.email.clone(),
                "url": url
            })),
            orchestrator,
        )
        .await;
        return Ok(CommonResponse::Done);
    }
    return Ok(CommonResponse::BadRequest);
}

pub async fn create_new_session(
    db: &DatabaseConnection,
    user: UserObject,
    user_agent: &str,
    ip: &str,
) -> Result<CommonResponse<SessionObject>, CommonError> {
    let session = create_session(db, user.id, user_agent.to_string(), ip.to_string()).await?;
    let session_object = create_session_object(user, session.pid);
    Ok(common_response::ok(session_object))
}
pub fn create_session_object(user: UserObject, session_pid: Uuid) -> SessionObject {
    SessionObject {
        session: session_pid,
        user,
    }
}

pub fn create_magic_link_token(user: &UserObject) -> Result<String, CommonError> {
    crypto::jwt_sign(user, 15 * 60, Some("MAGIC_LINK_SECRET"))
}

pub fn verify_magic_link_token(token: &str) -> Result<UserObject, CommonError> {
    crypto::jwt_verify(token, Some("MAGIC_LINK_SECRET"))
}

pub fn create_session_token(session: &SessionObject) -> Result<String, CommonError> {
    crypto::jwt_sign(session, 24 * 60 * 60, None)
}

pub fn verify_session_token(token: &String) -> Result<SessionObject, CommonError> {
    crypto::jwt_verify(token, None)
}

pub fn validate_password(password: &str) -> Result<bool, CommonError> {
    // minimum 8 charactes to 50 char
    logger::debug(&format!("Password: {}", password.len()));
    if password.len() < 8 || password.len() > 50 {
        return Err(CommonError::from(
            "Password should be between 8 to 50 characters".to_owned(),
        ));
    }
    // Should contain aleast 1 upper case
    if !regex::Regex::new("[A-Z]").unwrap().is_match(&password) {
        return Err(CommonError::from(
            "Password should contain aleast 1 upper case".to_owned(),
        ));
    }
    // Should contain atleast 1 lower case
    if !regex::Regex::new("[a-z]").unwrap().is_match(&password) {
        return Err(CommonError::from(
            "Password should contain atleast 1 lower case".to_owned(),
        ));
    }
    // Should contain atleast 1 num
    if !regex::Regex::new("[0-9]").unwrap().is_match(&password) {
        return Err(CommonError::from(
            "Password should contain atleast 1 num".to_owned(),
        ));
    }
    // Should contain atleast 1 special character
    if !regex::Regex::new("[\\!\\@\\#\\$\\%\\^\\&\\*]")
        .unwrap()
        .is_match(&password)
    {
        return Err(CommonError::from(
            "Password should contain atleast 1 special character".to_owned(),
        ));
    }

    // Should not contain spaces or new lines
    if regex::Regex::new("/[\\s\\n]/g")
        .unwrap()
        .is_match(&password)
    {
        return Err(CommonError::from(
            "Password should not contain spaces or new lines".to_owned(),
        ));
    }

    Ok(true)
}
