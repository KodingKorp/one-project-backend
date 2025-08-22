use uuid::Uuid;

use crate::{
    bootstrap::AppState,
    capabilities::{
        background::JobHandler,
        iam::{
            entities::organisation_to_user_mapping::{Role, UserOrgStatus},
            repositories::{organisations, users},
        },
        lib::common_error::CommonError,
        logger,
    },
};

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

    async fn run(
        &self,
        job: &crate::capabilities::background::JobModel,
        app_state: Option<AppState>,
    ) -> Result<Option<String>, CommonError> {
        if app_state.is_none() {
            return Err(CommonError::from("App state not found".to_owned()));
        }
        let app_state = app_state.unwrap();
        logger::debug(&format!(
            "[bg][iam][handler] Running register side effect handler for job {:?}",
            job
        ));
        let payload: RegisteredUser = serde_json::from_str(&job.payload.clone().unwrap()).unwrap();
        let user = users::find_user_by_pid(&app_state.db, payload.pid)
            .await?
            .ok_or_else(|| CommonError::from("User not found".to_owned()))?;
        logger::info(&format!(
            "[bg][iam][handler] Registering user with email {}",
            user.email
        ));
        // create org for user
        let org = organisations::create_organisation(&app_state.db, None).await?;
        let org = match org {
            Some(org) => org,
            None => return Err(CommonError::from("Organisation not found".to_owned())),
        };
        // add user to org
        let mapping = organisations::add_user_to_organisation(
            &app_state.db,
            user.id,
            org.id,
            Role::Admin,
            UserOrgStatus::Active,
        )
        .await?;
        if mapping.is_none() {
            return Err(CommonError::from("Mapping not found".to_owned()));
        }
        logger::info(&format!(
            "[bg][iam][handler] User {} registered to organisation {}",
            user.email, org.id
        ));
        Ok(None)
    }
}
