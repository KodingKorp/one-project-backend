// Public utils for other services
// allow unused in this file
#![allow(unused)]

use poem::session::Session;

use crate::logger;

pub use super::constants;

pub use super::{
    entities::organisation_to_user_mapping::Role,
    objects::{OrganisationObject, SessionObject, UserObject, UserOrgMappingObject},
};

pub use super::entities::{
    organisation_to_user_mapping::Model as UserOrgMappingModel, organisations::Model as OrgModel,
    session::Model as SessionModel, users::Model as UserModel,
};

/// Get current logged in user or panic if user not found
pub fn get_current_user(session: &Session) -> UserObject {
    let session_data = session.get::<SessionObject>(constants::SESSION_KEY_NAME);
    if session_data.is_none() {
        logger::error("Session data not found");
        panic!("Session data not found");
    }
    let session_object = session_data.unwrap();
    return session_object.user.clone();
}

/// Get current organisation object and none if not found
pub fn get_current_organisation(session: &Session) -> Option<OrganisationObject> {
    session.get::<OrganisationObject>(constants::ORG_KEY_NAME)
}

/// Get current user id,  role and org id
/// Return None if not found
/// Return Some((user_id, role, org_id))
pub fn get_current_user_access_info(session: &Session) -> Option<(i32, Role, i32)> {
    let session_data = session.get::<UserOrgMappingObject>(constants::USER_MAPPING_KEY_NAME);
    if session_data.is_none() {
        logger::error("Session data not found");
        return None;
    }
    let session_object = session_data.unwrap();
    // parse role
    let role = match Role::try_from(session_object.role) {
        Ok(role) => role,
        Err(_) => {
            logger::error("Role not found");
            return None;
        }
    };
    return Some((session_object.user_id, role, session_object.org_id));
}
