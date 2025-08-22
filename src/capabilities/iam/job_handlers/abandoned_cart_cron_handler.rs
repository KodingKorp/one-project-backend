use std::str::FromStr;

use crate::{
    bootstrap::AppState,
    capabilities::{background::JobHandler, lib::common_error::CommonError, logger},
};

pub struct AbandonedCartHandler;

impl AbandonedCartHandler {
    pub fn new() -> Self {
        Self {}
    }
    pub fn get_pattern(&self) -> cron::Schedule {
        cron::Schedule::from_str("0 */5 * * * *").unwrap()
    }
}

#[async_trait::async_trait]
impl JobHandler for AbandonedCartHandler {
    fn get_job_id(&self) -> &str {
        "iam_abandoned_cart"
    }

    async fn run(
        &self,
        job: &crate::capabilities::background::JobModel,
        _: Option<AppState>,
    ) -> Result<Option<String>, CommonError> {
        logger::debug(&format!(
            "[bg][iam][iam_abandoned_cart] Running abandoned cart handler for job {:?}",
            job
        ));
        logger::info(&format!(
            "[bg][iam][iam_abandoned_cart] Triggered Abandoned cart handler at {}",
            chrono::Local::now()
        ));
        Ok(None)
    }
}
