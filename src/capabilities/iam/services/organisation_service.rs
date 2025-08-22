use super::super::entities::{
    organisation_to_user_mapping::Model as UserOrgMappingModel, user_login::LoginStrategy,
    users::Model as UserModel,
};
use super::auth_service::{self, get_session_object};
use crate::capabilities::background::orchestrator;
use crate::capabilities::crypto;
use crate::capabilities::iam::constants;
use crate::capabilities::iam::entities::organisation_to_user_mapping::{Role, UserOrgStatus};
use crate::capabilities::iam::entities::organisations::OrganisationStatus;
use crate::capabilities::iam::objects::{
    OrganisationObject, SessionObject, UserObject, UserOrgMappingObject, UserWithMappingObject,
};
use crate::capabilities::iam::repositories::organisations::update_organisation_name;
use crate::capabilities::iam::repositories::{organisations::*, users};
use crate::capabilities::lib::common_response;
use crate::capabilities::lib::{common_error::CommonError, common_response::CommonResponse};
use crate::capabilities::notifications::service::NotificationService;
use crate::{config, logger};
use poem::session::Session;
use sea_orm::DatabaseConnection;

pub async fn get_current_organisation(
    session: &Session,
) -> Result<CommonResponse<OrganisationObject>, CommonError> {
    match session.get::<OrganisationObject>(constants::ORG_KEY_NAME) {
        Some(org) => Ok(common_response::ok(org)),
        None => Ok(CommonResponse::NotFound),
    }
}

pub async fn switch_organisation(
    db: &DatabaseConnection,
    session_store: &Session,
    session: &SessionObject,
    org_id: i32,
) -> Result<CommonResponse<OrganisationObject>, CommonError> {
    // check if org exists and user is part of it
    let org = match find_organisation_by_id(db, org_id).await {
        Ok(org) => org,
        Err(e) => {
            logger::error(&format!("Error finding organisation: {}", e));
            return Ok(CommonResponse::NotFound);
        }
    };
    if org.is_none() {
        logger::error("Organisation not found");
        return Ok(CommonResponse::NotFound);
    }
    let org = org.unwrap();
    let user_org = match get_user_mapping_for_organisation(db, org.id, session.user.id).await {
        Ok(Some(user_org)) => user_org,
        Ok(None) => {
            logger::error("User not part of organisation");
            return Ok(CommonResponse::Forbidden);
        }
        Err(e) => {
            logger::error(&format!("Error finding user organisation: {}", e));
            return Ok(CommonResponse::InternalServerError);
        }
    };
    match user_org.status {
        UserOrgStatus::Active => {
            // set the active organisation in the session
            let org = OrganisationObject::from(org);
            session_store.set(constants::ORG_KEY_NAME, &org);
            session_store.set(constants::USER_MAPPING_KEY_NAME, &user_org);
            Ok(common_response::ok(org))
        }
        UserOrgStatus::Invited => {
            logger::error("User organisation mapping is pending");
            Ok(CommonResponse::Forbidden)
        }
        UserOrgStatus::Inactive => {
            logger::error("User organisation mapping is inactive");
            Ok(CommonResponse::Forbidden)
        }
    }
}

/// Invite a user to an organisation
/// Check if current user is admin of the organisation
/// If not, return error
/// If yes, invite user to organisation and send invite email via orchestrator
pub async fn invite_user(
    db: &DatabaseConnection,
    session_store: &Session,
    session: &SessionObject,
    email: String,
    role: String,
    orchestrator: orchestrator::BackgroundOrchestrator,
) -> Result<CommonResponse<UserOrgMappingObject>, CommonError> {
    let role = Role::try_from(role).map_err(|_| CommonError::new("Invalid role"))?;
    // check if org exists and user is part of it
    let org = match session_store.get::<OrganisationObject>(constants::ORG_KEY_NAME) {
        Some(org) => org,
        None => {
            logger::error("Organisation not found");
            return Ok(CommonResponse::NotFound);
        }
    };
    let user_org = match get_user_mapping_for_organisation(db, org.id, session.user.id).await {
        Ok(Some(user_org)) => user_org,
        Ok(None) => {
            logger::error("User not part of organisation");
            return Ok(CommonResponse::Forbidden);
        }
        Err(e) => {
            logger::error(&format!("Error finding user organisation: {}", e));
            return Ok(CommonResponse::InternalServerError);
        }
    };
    if user_org.role != Role::Admin {
        logger::error("User is not admin of organisation");
        return Ok(CommonResponse::Forbidden);
    }
    // check if user already exists
    let user = match users::find_user_by_email(db, email.clone()).await {
        Ok(user) => user,
        Err(e) => {
            logger::error(&format!("Error finding user: {}", e));
            return Ok(CommonResponse::InternalServerError);
        }
    };
    // if no user then register user with magic link login using auth_service
    let user = match user {
        Some(user) => user,
        None => {
            let user = auth_service::register(
                db,
                email.clone(),
                LoginStrategy::MagicLink,
                None,
                false,
                false,
                Some(orchestrator.clone()),
            )
            .await;
            match user {
                Ok(CommonResponse::Ok(user)) => {
                    let user: UserModel = match user.data.to_owned().try_into() {
                        Ok(user) => user,
                        Err(e) => {
                            logger::error(&format!("Error converting user: {}", e));
                            return Ok(CommonResponse::InternalServerError);
                        }
                    };
                    user
                }
                Err(e) => {
                    logger::error(&format!("Error registering user: {}", e));
                    return Ok(CommonResponse::InternalServerError);
                }
                _ => {
                    logger::error("Error registering user");
                    return Ok(CommonResponse::InternalServerError);
                }
            }
        }
    };
    // check if user is already part of the organisation
    match get_user_mapping_for_organisation(db, org.id, user.id).await {
        Ok(Some(_)) => {
            logger::error("User already part of organisation");
            return Ok(CommonResponse::Conflict);
        }
        Ok(None) => {
            logger::info("User not part of organisation");
        }
        Err(e) => {
            logger::error(&format!("Error finding user organisation: {}", e));
            return Ok(CommonResponse::InternalServerError);
        }
    };
    // invite user to organisation
    let invite = match invite_user_to_organisation(db, org.id, user.id, role).await {
        Ok(invite) => invite,
        Err(e) => {
            logger::error(&format!("Error inviting user: {}", e));
            return Ok(CommonResponse::InternalServerError);
        }
    };
    let token = create_invite_link_token(&invite.into())?;
    let url = config::get_env::<String>("BASE_ORG_INVITE_LINK") + &token;
    // send invite email via orchestrator
    let _ = NotificationService::send_mail(
        "invite_to_org",
        "You are invited to join an organisation",
        &user.email,
        &user.email,
        Some(serde_json::json!({
            "organisation": org.name,
            "url": url,
        })),
        orchestrator,
    )
    .await;
    Ok(CommonResponse::Conflict)
}

/// Accept invite to organisation
/// Parse token, check the status of the user, and org and the mapping in DB
/// If mapping status already is active, set cookies for mapping and organisation and session
/// If mapping status is inactive, return error (meaning user invite was recinded)
/// If mapping status is invited, set mapping status to active and set cookies for mapping and organisation and session
pub async fn accept_invite(
    db: &DatabaseConnection,
    session_store: &Session,
    token: String,
    user_agent: &str,
    ip: &str,
) -> Result<CommonResponse<UserOrgMappingObject>, CommonError> {
    let mapping = verify_invite_link_token(&token)?;
    let mapping = match get_mapping_by_id(db, mapping.org_id).await {
        Ok(Some(mapping)) => mapping,
        Ok(None) => {
            logger::error("User not found");
            return Ok(CommonResponse::NotFound);
        }
        Err(e) => {
            logger::error(&format!("Error finding user organisation mapping: {}", e));
            return Ok(CommonResponse::InternalServerError);
        }
    };

    if mapping.status == UserOrgStatus::Inactive {
        logger::error("User organisation mapping is inactive");
        return Ok(CommonResponse::NotFound);
    }
    // get organisation
    let org = match find_organisation_by_id(db, mapping.org_id).await {
        Ok(Some(org)) => org,
        Ok(None) => {
            logger::error("Organisation not found");
            return Ok(CommonResponse::NotFound);
        }
        Err(e) => {
            logger::error(&format!("Error finding organisation: {}", e));
            return Ok(CommonResponse::InternalServerError);
        }
    };
    if org.status == OrganisationStatus::Inactive {
        logger::error("Organisation is inactive");
        return Ok(CommonResponse::NotFound);
    }
    // get user
    let user = match users::find_user_by_id(db, mapping.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            logger::error("User not found");
            return Ok(CommonResponse::NotFound);
        }
        Err(e) => {
            logger::error(&format!("Error finding user: {}", e));
            return Ok(CommonResponse::InternalServerError);
        }
    };
    if mapping.status == UserOrgStatus::Active {
        logger::error("User organisation mapping is already active");
        // set the active organisation in the session
        let org = OrganisationObject::from(org);
        session_store.set(constants::ORG_KEY_NAME, &org);
        let mapping = UserOrgMappingObject::from(mapping);
        session_store.set(constants::USER_MAPPING_KEY_NAME, &mapping);
        let user = UserObject::from(user);
        let session_obj = auth_service::create_new_session(db, &user, user_agent, ip).await?;
        let CommonResponse::Ok(session_obj) = session_obj else {
            return Ok(CommonResponse::InternalServerError);
        };
        session_store.set(constants::SESSION_KEY_NAME, &session_obj.data);
        return Ok(CommonResponse::Conflict);
    }
    // activate mapping
    match activate_user_in_organisation(db, mapping.org_id, mapping.user_id).await {
        Ok(_) => {
            logger::info("User organisation mapping activated");
            let mapping = UserOrgMappingObject::from(mapping);
            Ok(common_response::ok(mapping))
        }
        Err(e) => {
            logger::error(&format!(
                "Error activating user organisation mapping: {}",
                e
            ));
            Ok(CommonResponse::InternalServerError)
        }
    }
}

/// Get input of mapping id
/// Check if user is part of the organisation
/// If not, return error
/// If yes, check if user is already invited
/// If yes, send invite email
/// If no, return error
/// If user is not part of the organisation, return error
/// If user is part of the organisation, check if user is already invited
/// If yes, return error
/// If no, send invite email
pub async fn resend_invite(
    db: &DatabaseConnection,
    current_user_mapping: &UserOrgMappingObject,
    mapping_id: i32,
    orchestrator: orchestrator::BackgroundOrchestrator,
) -> Result<CommonResponse<UserOrgMappingObject>, CommonError> {
    // get mapping object
    let mapping = match get_mapping_by_id(db, mapping_id).await {
        Ok(Some(mapping)) => mapping,
        Ok(None) => {
            logger::error("Mapping not found");
            return Ok(CommonResponse::NotFound);
        }
        Err(e) => {
            logger::error(&format!("Error finding user organisation mapping: {}", e));
            return Ok(CommonResponse::InternalServerError);
        }
    };
    // check if user is part of the organisation
    let current_user_mapping = UserOrgMappingModel::try_from(current_user_mapping.clone())?;
    if current_user_mapping.org_id != mapping.org_id {
        logger::error("User not part of organisation");
        return Ok(CommonResponse::Forbidden);
    }
    // check if current user is admin
    if current_user_mapping.role != Role::Admin {
        logger::error("User is not admin of organisation");
        return Ok(CommonResponse::Forbidden);
    }
    // check if mapping is not in Invited status
    if mapping.status != UserOrgStatus::Invited {
        logger::error("User organisation mapping is not in invited status");
        return Ok(CommonResponse::Forbidden);
    }

    // get user
    let user = match users::find_user_by_id(db, mapping.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            logger::error("User not found");
            return Ok(CommonResponse::NotFound);
        }
        Err(e) => {
            logger::error(&format!("Error finding user: {}", e));
            return Ok(CommonResponse::InternalServerError);
        }
    };
    // send invite email
    let mapping_obj = UserOrgMappingObject::from(mapping);
    let token = create_invite_link_token(&mapping_obj)?;
    let url = config::get_env::<String>("BASE_ORG_INVITE_LINK") + &token;
    let _ = NotificationService::send_mail(
        "invite_to_org",
        "You are invited to join an organisation",
        &user.email,
        &user.email,
        Some(serde_json::json!({
            "organisation": mapping_obj.org_id,
            "url": url,
        })),
        orchestrator,
    )
    .await;
    logger::info("Invite email sent");
    Ok(common_response::ok(mapping_obj))
}

/// get mapping id and mark org user as inactive
/// check if current user is admin of the organisation
/// If not, return error
/// If yes, mark mapping as inactive if mapping exists
pub async fn mark_user_inactive(
    db: &DatabaseConnection,
    current_user_mapping: &UserOrgMappingObject,
    mapping_id: i32,
) -> Result<CommonResponse<UserOrgMappingObject>, CommonError> {
    // get mapping object
    let mapping = match get_mapping_by_id(db, mapping_id).await {
        Ok(Some(mapping)) => mapping,
        Ok(None) => {
            logger::error("Mapping not found");
            return Ok(CommonResponse::NotFound);
        }
        Err(e) => {
            logger::error(&format!("Error finding user organisation mapping: {}", e));
            return Ok(CommonResponse::InternalServerError);
        }
    };
    // check if user is part of the organisation
    let current_user_mapping = UserOrgMappingModel::try_from(current_user_mapping.clone())?;
    if current_user_mapping.org_id != mapping.org_id {
        logger::error("User not part of organisation");
        return Ok(CommonResponse::Forbidden);
    }
    // check if current user is admin
    if current_user_mapping.role != Role::Admin {
        logger::error("User is not admin of organisation");
        return Ok(CommonResponse::Forbidden);
    }
    // mark mapping as inactive
    match remove_user_from_organisation(db, mapping.org_id, mapping.user_id).await {
        Ok(Some(mapping)) => {
            logger::info("User organisation mapping marked as inactive");
            let mapping = UserOrgMappingObject::from(mapping);
            Ok(common_response::ok(mapping))
        }
        Ok(None) => {
            logger::error("User organisation mapping not found");
            Ok(CommonResponse::NotFound)
        }
        Err(e) => {
            logger::error(&format!(
                "Error marking user organisation mapping as inactive: {}",
                e
            ));
            Ok(CommonResponse::InternalServerError)
        }
    }
}

/// get current user's mapping from session
pub async fn get_current_user_mapping(
    session: &Session,
) -> Result<CommonResponse<UserOrgMappingObject>, CommonError> {
    match session.get::<UserOrgMappingObject>(constants::USER_MAPPING_KEY_NAME) {
        Some(mapping) => Ok(common_response::ok(mapping)),
        None => {
            logger::error("User mapping not found");
            Ok(CommonResponse::NotFound)
        }
    }
}

/// get paginated list of users in organisation with their mappings, does not require admin
pub async fn get_users_in_organisation(
    db: &DatabaseConnection,
    session_store: &Session,
    status: UserOrgStatus,
    page: u64,
    page_size: u64,
) -> Result<CommonResponse<Vec<UserWithMappingObject>>, CommonError> {
    // get organisation from session
    let org = match session_store.get::<OrganisationObject>(constants::ORG_KEY_NAME) {
        Some(org) => org,
        None => {
            logger::error("Organisation not found");
            return Ok(CommonResponse::NotFound);
        }
    };
    // get users in organisation
    let users = match get_users_in_organisation_by_status(db, org.id, status, page, page_size).await
    {
        Ok(users) => users,
        Err(e) => {
            logger::error(&format!("Error getting users in organisation: {}", e));
            return Ok(CommonResponse::InternalServerError);
        }
    };
    Ok(common_response::ok(
        users
            .into_iter()
            .map(UserWithMappingObject::from)
            .collect(),
    ))
}

/// get all user's organisations
pub async fn get_user_organisations(
    db: &DatabaseConnection,
    session_store: &Session,
) -> Result<CommonResponse<Vec<OrganisationObject>>, CommonError> {
    // get user from session
    let session = get_session_object(session_store);
    let user = session.user;
    // get organisations for user
    let orgs = match get_all_users_active_organisations(db, user.id).await {
        Ok(orgs) => orgs,
        Err(e) => {
            logger::error(&format!("Error getting organisations for user: {}", e));
            return Ok(CommonResponse::InternalServerError);
        }
    };
    Ok(common_response::ok(
        orgs.into_iter()
            .map(OrganisationObject::from)
            .collect(),
    ))
}

/// deactivate current organisation
/// check if user is admin of the organisation
/// If not, return error
pub async fn deactivate_current_organisation(
    db: &DatabaseConnection,
    session_store: &Session,
) -> Result<CommonResponse<OrganisationObject>, CommonError> {
    // get organisation from session
    let org = match session_store.get::<OrganisationObject>(constants::ORG_KEY_NAME) {
        Some(org) => org,
        None => {
            logger::error("Organisation not found");
            return Ok(CommonResponse::NotFound);
        }
    };
    // get user mapping
    let user_mapping =
        match session_store.get::<UserOrgMappingObject>(constants::USER_MAPPING_KEY_NAME) {
            Some(mapping) => UserOrgMappingModel::try_from(mapping)?,
            None => {
                logger::error("User mapping not found");
                return Ok(CommonResponse::NotFound);
            }
        };
    // check if user is admin of the organisation
    if user_mapping.role != Role::Admin {
        logger::error("User is not admin of organisation");
        return Ok(CommonResponse::Forbidden);
    }
    // deactivate organisation
    match deactivate_organisation(db, org.id).await {
        Ok(_) => {
            logger::info("Organisation deactivated");
            Ok(common_response::ok(org))
        }
        Err(e) => {
            logger::error(&format!("Error deactivating organisation: {}", e));
            Ok(CommonResponse::InternalServerError)
        }
    }
}

/// update organisation name
/// check if user is admin of the organisation
/// If not, return error
/// If yes, update organisation name
pub async fn update_org_name(
    db: &DatabaseConnection,
    session_store: &Session,
    name: String,
) -> Result<CommonResponse<OrganisationObject>, CommonError> {
    // get organisation from session
    let org = match session_store.get::<OrganisationObject>(constants::ORG_KEY_NAME) {
        Some(org) => org,
        None => {
            logger::error("Organisation not found");
            return Ok(CommonResponse::NotFound);
        }
    };
    // get user mapping
    let user_mapping =
        match session_store.get::<UserOrgMappingObject>(constants::USER_MAPPING_KEY_NAME) {
            Some(mapping) => UserOrgMappingModel::try_from(mapping)?,
            None => {
                logger::error("User mapping not found");
                return Ok(CommonResponse::NotFound);
            }
        };
    // check if user is admin of the organisation
    if user_mapping.role != Role::Admin {
        logger::error("User is not admin of organisation");
        return Ok(CommonResponse::Forbidden);
    }
    // update organisation name
    match update_organisation_name(db, org.id, name).await {
        Ok(org) => {
            logger::info("Organisation name updated");
            Ok(common_response::ok(OrganisationObject::from(org)))
        }
        Err(e) => {
            logger::error(&format!("Error updating organisation name: {}", e));
            Ok(CommonResponse::InternalServerError)
        }
    }
}

/* PRIVATE FUNCTIONS */
fn create_invite_link_token(mapping: &UserOrgMappingObject) -> Result<String, CommonError> {
    // 7 days expiry
    crypto::jwt_sign(mapping, 7 * 24 * 60, Some("ORG_INVITE_SECRET"))
}

fn verify_invite_link_token(token: &str) -> Result<UserOrgMappingObject, CommonError> {
    // verify token
    let secret = config::get_env::<String>("ORG_INVITE_SECRET");
    match crypto::jwt_verify::<UserOrgMappingObject>(token, Some(secret.as_str())) {
        Ok(mapping) => {
            Ok(mapping)
        }
        Err(e) => {
            logger::error(&format!("Error verifying token: {}", e));
            Err(CommonError::new("Token invalid"))
        }
    }
}
