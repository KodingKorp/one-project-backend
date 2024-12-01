use uuid::Uuid;

use crate::{bootstrap::AppState, capabilities::{background::JobHandler, iam::users, lib::common_error::CommonError, logger}};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RegisteredUser {
    pub pid: Uuid,
}

pub struct RegisterSideEffectHandler;

impl RegisterSideEffectHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl JobHandler for RegisterSideEffectHandler {
    fn get_job_id(&self) -> &str {
        "iam_register_side_effect"
    }

    async fn run(&self, job: &crate::capabilities::background::JobModel, app_state: Option<AppState>) -> Result<Option<String>, CommonError> {
        if app_state.is_none() {
            return Err(CommonError::from("App state not found".to_owned()));
        }
        let payload: RegisteredUser = serde_json::from_str(&job.payload.clone().unwrap()).unwrap();
        let user = users::find_user_by_pid(&app_state.unwrap().db, payload.pid).await?.ok_or_else(|| CommonError::from("User not found".to_owned()))?;
        logger::info(&format!("[bg][iam][handler] Registering user with email {}", user.email));
        Ok(None)
    }
}