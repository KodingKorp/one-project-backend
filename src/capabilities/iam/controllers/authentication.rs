use crate::{
    bootstrap::AppState,
    capabilities::{
        background,
        iam::{
            entities::user_login::LoginStrategy,
            objects::{SessionObject, UserObject},
            services::auth_service,
        },
        lib::common_response::CommonResponse,
        logger,
    },
};
use poem::{session::Session, web::Data, Request};
use poem_openapi::{param::Path, payload::Json, types::Email, Object, OpenApi};

use ammonia::clean;

#[derive(Debug, Object, Clone, Eq, PartialEq)]
pub struct CreateUser {
    email: Email,
}

#[derive(Debug, Object, Clone, Eq, PartialEq)]
pub struct CreateUserWithPassword {
    pub email: Email,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Debug, Object, Clone, Eq, PartialEq)]
pub struct LoginWithPassword {
    pub email: Email,
    pub password: String,
}

#[derive(Default)]
pub struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/auth/register-link", method = "post")]
    pub async fn register(
        &self,
        state: Data<&AppState>,
        orchestrator: Data<&background::orchestrator::BackgroundOrchestrator>,
        payload: Json<CreateUser>,
    ) -> CommonResponse<UserObject> {
        match auth_service::register(
            &state.db,
            clean(payload.email.as_str()),
            LoginStrategy::MagicLink,
            None,
            true,
            true,
            Some(orchestrator.0.clone()),
        )
        .await
        {
            Ok(user) => user,
            Err(_) => CommonResponse::InternalServerError,
        }
    }

    #[oai(path = "/auth/register-pass", method = "post")]
    pub async fn register_pass(
        &self,
        state: Data<&AppState>,
        orchestrator: Data<&background::orchestrator::BackgroundOrchestrator>,
        payload: Json<CreateUserWithPassword>,
    ) -> CommonResponse<UserObject> {
        logger::info(&format!(
            "Registering user with email: {}",
            *payload.email
        ));
        if payload.password != payload.confirm_password {
            return CommonResponse::BadRequest;
        }
        logger::info(&format!(
            "Registering user with email: {}",
            *payload.email
        ));
        match auth_service::register(
            &state.db,
            clean(payload.email.as_str()),
            LoginStrategy::Password,
            Some(payload.password.clone()),
            true,
            true,
            Some(orchestrator.0.clone()),
        )
        .await
        {
            Ok(user) => user,
            Err(_) => CommonResponse::InternalServerError,
        }
    }

    #[oai(path = "/auth/send-magic-link", method = "post")]
    pub async fn send_magic_link(
        &self,
        state: Data<&AppState>,
        orchestrator: Data<&background::orchestrator::BackgroundOrchestrator>,
        payload: Json<CreateUser>,
    ) -> CommonResponse<String> {
        match auth_service::send_magic_link(
            &state.db,
            orchestrator.0.clone(),
            clean(payload.email.as_str()),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => CommonResponse::InternalServerError,
        }
    }

    #[oai(path = "/auth/login-pass", method = "post")]
    pub async fn login_pass(
        &self,
        req: &Request,
        state: Data<&AppState>,
        payload: Json<LoginWithPassword>,
        session_store: &Session,
    ) -> CommonResponse<SessionObject> {
        match auth_service::login_password(
            &state.db,
            clean(payload.email.as_str()),
            payload.password.clone(),
        )
        .await
        {
            Ok(CommonResponse::Ok(result)) => {
                let user_agent = req.header("User-Agent").unwrap_or_default();
                let ip = req.remote_addr().to_string();
                return auth_service::create_api_session(
                    &state.db,
                    session_store,
                    &result.0.data,
                    &clean(user_agent),
                    &ip,
                )
                .await;
            }
            Ok(_) => CommonResponse::InternalServerError,
            Err(_) => CommonResponse::InternalServerError,
        }
    }

    #[oai(path = "/auth/login-link/:token", method = "post")]
    pub async fn login_link(
        &self,
        req: &Request,
        state: Data<&AppState>,
        token: Path<String>,
        session_store: &Session,
    ) -> CommonResponse<SessionObject> {
        match auth_service::login_magic_link(&state.db, token.0.to_string()).await {
            Ok(CommonResponse::Ok(result)) => {
                let user_agent = req.header("User-Agent").unwrap_or_default();
                let ip = req.remote_addr().to_string();
                return auth_service::create_api_session(
                    &state.db,
                    session_store,
                    &result.0.data,
                    &clean(user_agent),
                    &ip,
                )
                .await;
            }
            Ok(_) => CommonResponse::InternalServerError,
            Err(_) => CommonResponse::InternalServerError,
        }
    }

    #[oai(path = "/auth/logout", method = "post")]
    pub async fn logout(&self, session_store: &Session) -> CommonResponse<String> {
        session_store.clear();
        CommonResponse::Done
    }
}
