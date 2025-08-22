use poem::{session::Session, web::Data};
use poem_openapi::OpenApi;

use crate::{
    bootstrap::AppState,
    capabilities::{
        iam::{objects::FullSessionInfo, services::auth_service},
        lib::common_response::{self, CommonResponse},
        logger,
    },
};

#[derive(Default)]
pub struct Api;

#[OpenApi]
impl Api {
    /// Get current logged in user
    #[oai(path = "/users/me", method = "get")]
    pub async fn get_current_user(
        &self,
        state: Data<&AppState>,
        session_store: Data<&Session>,
    ) -> CommonResponse<FullSessionInfo> {
        let result = auth_service::revalidate_session(&state.db, &session_store).await;
        match result {
            Ok(full_session) => common_response::ok(full_session),
            Err(err) => {
                logger::error(&format!("Failed to revalidate user session: {}", err));
                CommonResponse::InternalServerError
            }
        }
    }
}
