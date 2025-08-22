use poem::{session::Session, web::Data, Request};
use poem_openapi::{
    param::{Path, Query},
    Object, OpenApi,
};

use crate::{
    bootstrap::AppState,
    capabilities::{
        background,
        iam::{
            entities::organisation_to_user_mapping::UserOrgStatus,
            objects::{UserOrgMappingObject, UserWithMappingObject},
            services::{auth_service, organisation_service},
        },
        lib::common_response::{self, CommonResponse},
        logger,
    },
};
use ammonia::clean;
#[derive(Debug, Object, Clone, Eq, PartialEq)]
pub struct SwitchOrganisation {
    pub organisation_id: i32,
}

#[derive(Debug, Object, Clone, Eq, PartialEq)]
pub struct InviteUser {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Object, Clone, Eq, PartialEq)]
pub struct UpdateOrgName {
    pub name: String,
}

#[derive(Default)]
pub struct Api;

#[OpenApi]
impl Api {
    /// get current organisation
    #[oai(path = "/organisations/me", method = "get")]
    pub async fn get_current_organisation(
        &self,
        session: Data<&poem::session::Session>,
    ) -> crate::capabilities::lib::common_response::CommonResponse<
        crate::capabilities::iam::objects::OrganisationObject,
    > {
        match organisation_service::get_current_organisation(session.0).await {
            Ok(res) => res,
            Err(err) => {
                crate::capabilities::logger::error(&format!(
                    "Error getting current organisation: {}",
                    err
                ));
                crate::capabilities::lib::common_response::CommonResponse::<
                    crate::capabilities::iam::objects::OrganisationObject,
                >::InternalServerError
            }
        }
    }

    /// switch to selected organisation
    #[oai(path = "/organisations/switch", method = "post")]
    pub async fn switch_organisation(
        &self,
        state: Data<&AppState>,
        session: Data<&Session>,
        payload: poem_openapi::payload::Json<SwitchOrganisation>,
    ) -> common_response::CommonResponse<crate::capabilities::iam::objects::OrganisationObject>
    {
        let session_data = auth_service::get_session_object(session.0);
        match organisation_service::switch_organisation(
            &state.0.db,
            session.0,
            &session_data,
            payload.0.organisation_id,
        )
        .await
        {
            Ok(res) => res,
            Err(err) => {
                logger::error(&format!("Error switching organisation: {}", err));
                common_response::CommonResponse::InternalServerError
            }
        }
    }

    /// invite user to organisation
    #[oai(path = "/organisations/invite", method = "post")]
    pub async fn invite_user(
        &self,
        state: Data<&AppState>,
        session: Data<&Session>,
        payload: poem_openapi::payload::Json<InviteUser>,
        orchestrator: Data<&background::orchestrator::BackgroundOrchestrator>,
    ) -> common_response::CommonResponse<UserOrgMappingObject> {
        let session_data = auth_service::get_session_object(session.0);
        match organisation_service::invite_user(
            &state.0.db,
            session.0,
            &session_data,
            clean(payload.0.email.as_str()),
            clean(&payload.0.role),
            orchestrator.0.clone(),
        )
        .await
        {
            Ok(res) => res,
            Err(err) => {
                logger::error(&format!("Error inviting user: {}", err));
                common_response::CommonResponse::InternalServerError
            }
        }
    }

    /// get paginated users in organisation
    #[oai(path = "/organisations/users", method = "get")]
    pub async fn get_users(
        &self,
        state: Data<&AppState>,
        session: Data<&Session>,
        page: Query<Option<u64>>,
        page_size: Query<Option<u64>>,
        status: Query<Option<String>>,
    ) -> common_response::CommonResponse<Vec<UserWithMappingObject>> {
        let status = match status.0 {
            Some(s) => match UserOrgStatus::try_from(s) {
                Ok(status) => status,
                Err(_) => {
                    return common_response::CommonResponse::BadRequest;
                }
            },
            None => UserOrgStatus::Active,
        };
        match organisation_service::get_users_in_organisation(
            &state.0.db,
            session.0,
            status,
            page.unwrap_or(1),
            page_size.unwrap_or(10),
        )
        .await
        {
            Ok(res) => res,
            Err(err) => {
                logger::error(&format!("Error getting users: {}", err));
                common_response::CommonResponse::InternalServerError
            }
        }
    }
    /// get current user's mapping
    #[oai(path = "/organisations/me/mapping", method = "get")]
    pub async fn get_user_mapping(
        &self,
        session: Data<&Session>,
    ) -> common_response::CommonResponse<UserOrgMappingObject> {
        match organisation_service::get_current_user_mapping(session.0).await {
            Ok(res) => res,
            Err(err) => {
                logger::error(&format!("Error getting user mapping: {}", err));
                common_response::CommonResponse::InternalServerError
            }
        }
    }

    /// mark user as inactive
    #[oai(path = "/organisations/users/:mapping_id/inactive", method = "post")]
    pub async fn mark_user_inactive(
        &self,
        state: Data<&AppState>,
        session: Data<&Session>,
        mapping_id: Path<i32>,
    ) -> common_response::CommonResponse<UserOrgMappingObject> {
        let current_user_mapping = organisation_service::get_current_user_mapping(session.0).await;
        if current_user_mapping.is_err() {
            logger::error(&format!(
                "Error getting user mapping: {}",
                current_user_mapping.err().unwrap()
            ));
            return common_response::CommonResponse::InternalServerError;
        }
        let current_user_mapping = match current_user_mapping {
            Ok(CommonResponse::Ok(res)) => res.0.data,
            Err(err) => {
                logger::error(&format!("Error getting user mapping: {}", err));
                return common_response::CommonResponse::InternalServerError;
            }
            Ok(res) => {
                return res;
            }
        };

        match organisation_service::mark_user_inactive(
            &state.0.db,
            &current_user_mapping,
            mapping_id.0,
        )
        .await
        {
            Ok(res) => res,
            Err(err) => {
                logger::error(&format!("Error marking user as inactive: {}", err));
                common_response::CommonResponse::InternalServerError
            }
        }
    }

    /// resend invite
    #[oai(
        path = "/organisations/users/:mapping_id/resend-invite",
        method = "post"
    )]
    pub async fn resend_invite(
        &self,
        state: Data<&AppState>,
        session: Data<&Session>,
        mapping_id: Path<i32>,
        orchestrator: Data<&background::orchestrator::BackgroundOrchestrator>,
    ) -> common_response::CommonResponse<UserOrgMappingObject> {
        let current_user_mapping = organisation_service::get_current_user_mapping(session.0).await;
        if current_user_mapping.is_err() {
            logger::error(&format!(
                "Error getting user mapping: {}",
                current_user_mapping.err().unwrap()
            ));
            return common_response::CommonResponse::InternalServerError;
        }
        let current_user_mapping = match current_user_mapping {
            Ok(CommonResponse::Ok(res)) => res.0.data,
            Err(err) => {
                logger::error(&format!("Error getting user mapping: {}", err));
                return common_response::CommonResponse::InternalServerError;
            }
            Ok(res) => {
                return res;
            }
        };

        match organisation_service::resend_invite(
            &state.0.db,
            &current_user_mapping,
            mapping_id.0,
            orchestrator.0.clone(),
        )
        .await
        {
            Ok(res) => res,
            Err(err) => {
                logger::error(&format!("Error resending invite: {}", err));
                common_response::CommonResponse::InternalServerError
            }
        }
    }

    /// Get active organisation list
    #[oai(path = "/organisations/active", method = "get")]
    pub async fn get_active_organisations(
        &self,
        state: Data<&AppState>,
        session: Data<&Session>,
    ) -> common_response::CommonResponse<Vec<crate::capabilities::iam::objects::OrganisationObject>>
    {
        match organisation_service::get_user_organisations(&state.0.db, session.0).await {
            Ok(res) => res,
            Err(err) => {
                logger::error(&format!("Error getting active organisations: {}", err));
                common_response::CommonResponse::InternalServerError
            }
        }
    }

    /// Deactivate current organisation
    #[oai(path = "/organisations/deactivate", method = "post")]
    pub async fn deactivate_organisation(
        &self,
        state: Data<&AppState>,
        session: Data<&Session>,
    ) -> common_response::CommonResponse<crate::capabilities::iam::objects::OrganisationObject>
    {
        match organisation_service::deactivate_current_organisation(&state.0.db, session.0).await {
            Ok(res) => res,
            Err(err) => {
                logger::error(&format!("Error deactivating organisation: {}", err));
                common_response::CommonResponse::InternalServerError
            }
        }
    }

    /// update organisation name
    #[oai(path = "/organisations/update", method = "post")]
    pub async fn update_organisation(
        &self,
        state: Data<&AppState>,
        session: Data<&Session>,
        payload: poem_openapi::payload::Json<UpdateOrgName>,
    ) -> common_response::CommonResponse<crate::capabilities::iam::objects::OrganisationObject>
    {
        match organisation_service::update_org_name(&state.0.db, session.0, clean(&payload.0.name))
            .await
        {
            Ok(res) => res,
            Err(err) => {
                logger::error(&format!("Error updating organisation: {}", err));
                common_response::CommonResponse::InternalServerError
            }
        }
    }
}

#[derive(Debug, Object, Clone, Eq, PartialEq)]
pub struct AcceptInvite {
    pub token: String,
}

#[derive(Default)]
pub struct PublicApi;

#[OpenApi]
impl PublicApi {
    /// get current organisation
    #[oai(path = "/organisations/accept-invite", method = "post")]
    pub async fn accept_invite(
        &self,
        req: &Request,
        state: Data<&AppState>,
        session: Data<&poem::session::Session>,
        payload: poem_openapi::payload::Json<AcceptInvite>,
    ) -> crate::capabilities::lib::common_response::CommonResponse<UserOrgMappingObject> {
        let user_agent = req.header("User-Agent").unwrap_or_default();
        let ip = req.remote_addr().to_string();
        match organisation_service::accept_invite(
            &state.0.db,
            session.0,
            payload.0.token,
            user_agent,
            &ip,
        )
        .await
        {
            Ok(res) => res,
            Err(err) => {
                crate::capabilities::logger::error(&format!(
                    "Error getting current organisation: {}",
                    err
                ));
                crate::capabilities::lib::common_response::CommonResponse::InternalServerError
            }
        }
    }
}
