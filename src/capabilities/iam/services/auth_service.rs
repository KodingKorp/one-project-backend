use poem::session::Session;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::capabilities::background;
use crate::capabilities::background::orchestrator::RunJob;
use crate::capabilities::crypto::{self, hash_password, verify_password};
use crate::capabilities::iam::constants;
use crate::capabilities::iam::entities::{self, organisation_to_user_mapping};
use crate::capabilities::iam::job_handlers::register_side_effect_handler::RegisteredUser;
use crate::capabilities::iam::objects::{FullSessionInfo, UserOrgMappingObject};
use crate::capabilities::lib::common_error::CommonError;

use crate::capabilities::lib::common_response::{self, CommonResponse};
use crate::capabilities::{config, logger, notifications};

use self::notifications::service::NotificationService;

use super::super::entities::user_login::LoginStrategy;

use super::super::objects::{OrganisationObject, SessionObject, UserObject};
use super::super::repositories::{
    organisations,
    session::create_session,
    user_login::{find_strategy_value, update_or_create_user_login},
    users::{convert_model_to_object, create_user, find_user_by_email},
};

/// Register a new user
pub async fn register(
    db: &DatabaseConnection,
    email: String,
    strategy: LoginStrategy,
    mut strategy_value: Option<String>,
    trigger_side_effect: bool,
    send_email: bool,
    orchestrator: Option<background::orchestrator::BackgroundOrchestrator>,
) -> Result<CommonResponse<UserObject>, CommonError> {
    if trigger_side_effect || send_email {
        assert!(orchestrator.is_some(), "Orchestrator should be provided");
    }
    let user = find_user_by_email(db, email.clone()).await?;
    if user.is_some() {
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
        if strategy == LoginStrategy::MagicLink && send_email {
            let orchestrator = orchestrator.clone().unwrap();
            let token = create_magic_link_token(&user)?;
            let url = config::get_env::<String>("BASE_LOGIN_LINK") + &token;
            let _ = NotificationService::send_mail(
                "magic_link",
                "Magic Link",
                &user.email,
                &user.email,
                Some(serde_json::json!({
                    "first_name": user.email.clone(),
                    "url": url
                })),
                orchestrator,
            )
            .await;
        }
        if trigger_side_effect {
            let mut orchestrator = orchestrator.unwrap();
            orchestrator
                .run_job(RunJob {
                    queue: "iam".to_owned(),
                    delay: None,
                    job_type: background::JobType::Immediate,
                    payload: Some(
                        serde_json::to_string(&RegisteredUser { pid: user.pid }).unwrap(),
                    ),
                    max_retries: None,
                    name: "iam_register_side_effect".to_owned(),
                })
                .await;
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
    Ok(CommonResponse::BadRequest)
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
    Ok(CommonResponse::BadRequest)
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
            "magic_link",
            "Magic Link",
            &user.email,
            &user.email,
            Some(serde_json::json!({
                "first_name": user.email.clone(),
                "url": url
            })),
            orchestrator,
        )
        .await;
        return Ok(CommonResponse::Done);
    }
    Ok(CommonResponse::BadRequest)
}

pub async fn create_new_session(
    db: &DatabaseConnection,
    user: &UserObject,
    user_agent: &str,
    ip: &str,
) -> Result<CommonResponse<SessionObject>, CommonError> {
    let session = create_session(db, user.id, user_agent.to_string(), ip.to_string()).await?;
    let session_object = create_session_object(user, session.pid);
    Ok(common_response::ok(session_object))
}
pub fn create_session_object(user: &UserObject, session_pid: Uuid) -> SessionObject {
    SessionObject {
        session: session_pid,
        user: user.clone(),
        set_at: chrono::Utc::now().timestamp_millis(),
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

pub fn verify_session_token(token: &str) -> Result<SessionObject, CommonError> {
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
    if !regex::Regex::new("[A-Z]").unwrap().is_match(password) {
        return Err(CommonError::from(
            "Password should contain aleast 1 upper case".to_owned(),
        ));
    }
    // Should contain atleast 1 lower case
    if !regex::Regex::new("[a-z]").unwrap().is_match(password) {
        return Err(CommonError::from(
            "Password should contain atleast 1 lower case".to_owned(),
        ));
    }
    // Should contain atleast 1 num
    if !regex::Regex::new("[0-9]").unwrap().is_match(password) {
        return Err(CommonError::from(
            "Password should contain atleast 1 num".to_owned(),
        ));
    }
    // Should contain atleast 1 special character
    if !regex::Regex::new("[\\!\\@\\#\\$\\%\\^\\&\\*]")
        .unwrap()
        .is_match(password)
    {
        return Err(CommonError::from(
            "Password should contain atleast 1 special character".to_owned(),
        ));
    }

    // Should not contain spaces or new lines
    if regex::Regex::new("/[\\s\\n]/g").unwrap().is_match(password) {
        return Err(CommonError::from(
            "Password should not contain spaces or new lines".to_owned(),
        ));
    }

    Ok(true)
}

pub async fn create_api_session(
    db: &DatabaseConnection,
    session_store: &Session,
    user: &UserObject,
    user_agent: &str,
    ip: &str,
) -> CommonResponse<SessionObject> {
    match create_new_session(db, user, user_agent, ip).await {
        Ok(CommonResponse::Ok(session_object)) => {
            session_store.set(constants::SESSION_KEY_NAME, &session_object.0.data);
            if let Ok(Some(org)) =
                organisations::get_users_most_recent_organisation(db, session_object.data.user.id)
                    .await
            {
                let org = OrganisationObject::from(org);
                session_store.set(constants::ORG_KEY_NAME, &org);
                match organisations::get_user_mapping_for_organisation(
                    db,
                    org.id,
                    session_object.data.user.id,
                )
                .await
                {
                    Ok(Some(user_mapping)) => {
                        let user_mapping = UserOrgMappingObject::from(user_mapping);
                        session_store.set(constants::USER_MAPPING_KEY_NAME, &user_mapping);
                        return CommonResponse::Ok(session_object);
                    }
                    Ok(None) => {
                        logger::error("User mapping not found");
                        session_store.clear();
                        return CommonResponse::InternalServerError;
                    }
                    Err(_) => {
                        logger::error("Error fetching user mapping");
                        session_store.clear();
                        return CommonResponse::InternalServerError;
                    }
                }
            }
            session_store.clear();
            CommonResponse::InternalServerError
        }
        Ok(_) => CommonResponse::InternalServerError,
        Err(_) => CommonResponse::InternalServerError,
    }
}

/// get session object with guaranteed values
/// this function will panic if the session object is not found
/// this function should only be used in the context of a request
/// behind auth middleware
pub fn get_session_object(session: &Session) -> SessionObject {
    let session_data = session.get::<SessionObject>(constants::SESSION_KEY_NAME);
    if session_data.is_none() {
        logger::error("Session data not found");
        panic!("Session data not found");
    }
    session_data.unwrap()
}

pub async fn revalidate_session(
    db: &DatabaseConnection,
    session_store: &Session,
) -> Result<FullSessionInfo, CommonError> {
    let session = get_session_object(session_store);
    let max_session = config::get_env::<i64>("MAX_SESSION_DURATION");
    if max_session > 0 {
        let now = chrono::Utc::now().timestamp_millis();
        if now - session.set_at > max_session {
            // check data from DB and update session
            let user = find_user_by_email(db, session.user.email.clone()).await?;
            if let Some(user) = user {
                let user = convert_model_to_object(user);
                // get organisation
                let org = session_store.get::<OrganisationObject>(constants::ORG_KEY_NAME);
                if let Some(org) = org {
                    // get org from db
                    let mut most_recent_org = false;
                    let org = match organisations::find_organisation_by_id(db, org.id).await? {
                        Some(org) => {
                            if org.status != entities::organisations::OrganisationStatus::Active {
                                logger::error("Organisation is not active");
                                match organisations::get_users_most_recent_organisation(
                                    db,
                                    session.user.id,
                                )
                                .await?
                                {
                                    Some(org) => {
                                        most_recent_org = true;
                                        OrganisationObject::from(org)
                                    }
                                    None => {
                                        logger::error("Organisation not found");
                                        session_store.remove(constants::ORG_KEY_NAME);
                                        session_store.remove(constants::USER_MAPPING_KEY_NAME);
                                        return Ok(FullSessionInfo::new(session, None, None));
                                    }
                                }
                            } else {
                                OrganisationObject::from(org)
                            }
                        }
                        None => {
                            logger::error("Organisation not found");
                            return Ok(FullSessionInfo::new(session, None, None));
                        }
                    };

                    // get user mapping
                    let mapping =
                        organisations::get_user_mapping_for_organisation(db, org.id, user.id).await;
                    let mapping = match mapping {
                        Ok(Some(mapping)) => {
                            if mapping.status != organisation_to_user_mapping::UserOrgStatus::Active
                            {
                                if !most_recent_org {
                                    let org =
                                        match organisations::get_users_most_recent_organisation(
                                            db,
                                            session.user.id,
                                        )
                                        .await?
                                        {
                                            Some(org) => OrganisationObject::from(org),
                                            None => {
                                                logger::error("Organisation not found");
                                                session_store.remove(constants::ORG_KEY_NAME);
                                                session_store
                                                    .remove(constants::USER_MAPPING_KEY_NAME);
                                                return Ok(FullSessionInfo::new(
                                                    session, None, None,
                                                ));
                                            }
                                        };
                                    session_store.set(constants::ORG_KEY_NAME, &org);
                                    let mapping = organisations::get_user_mapping_for_organisation(
                                        db, org.id, user.id,
                                    )
                                    .await;
                                    if let Ok(Some(mapping)) = mapping {
                                        let mapping = UserOrgMappingObject::from(mapping);
                                        session_store
                                            .set(constants::USER_MAPPING_KEY_NAME, &mapping);
                                        return Ok(FullSessionInfo::new(
                                            session,
                                            Some(org),
                                            Some(mapping),
                                        ));
                                    } else {
                                        logger::error("User mapping not found");
                                        session_store.remove(constants::ORG_KEY_NAME);
                                        session_store.remove(constants::USER_MAPPING_KEY_NAME);
                                        return Ok(FullSessionInfo::new(session, None, None));
                                    }
                                } else {
                                    logger::error("User mapping is not active");
                                    session_store.remove(constants::ORG_KEY_NAME);
                                    session_store.remove(constants::USER_MAPPING_KEY_NAME);
                                    return Ok(FullSessionInfo::new(session, None, None));
                                }
                            }
                            UserOrgMappingObject::from(mapping)
                        }
                        Ok(None) => {
                            logger::error("User mapping not found");
                            session_store.remove(constants::ORG_KEY_NAME);
                            session_store.remove(constants::USER_MAPPING_KEY_NAME);
                            return Ok(FullSessionInfo::new(session, None, None));
                        }
                        Err(_) => {
                            logger::error("Error fetching user mapping");
                            session_store.remove(constants::ORG_KEY_NAME);
                            session_store.remove(constants::USER_MAPPING_KEY_NAME);
                            return Ok(FullSessionInfo::new(session, None, None));
                        }
                    };
                    return Ok(FullSessionInfo::new(session, Some(org), Some(mapping)));
                } else {
                    session_store.remove(constants::ORG_KEY_NAME);
                    session_store.remove(constants::USER_MAPPING_KEY_NAME);
                    return Ok(FullSessionInfo::new(session, None, None));
                }
            }
        } else {
            // get values from session store and return a FullSessionInfo object
            let org = session_store.get::<OrganisationObject>(constants::ORG_KEY_NAME);
            let mapping =
                session_store.get::<UserOrgMappingObject>(constants::USER_MAPPING_KEY_NAME);
            return Ok(FullSessionInfo::existing_session(session, org, mapping));
        }
    }
    Err(CommonError::new("Max session not set"))
}
