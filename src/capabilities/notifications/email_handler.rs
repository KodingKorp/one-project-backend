use crate::{
    bootstrap::AppState,
    capabilities::{background::JobHandler, logger},
};

use super::objects::NotificationMailMessage;

pub struct EmailHandler {
    mailer: crate::capabilities::mailer::Mailer,
}
impl EmailHandler {
    pub fn new() -> Self {
        Self {
            mailer: crate::capabilities::mailer::Mailer::new(),
        }
    }
}
#[async_trait::async_trait]
impl JobHandler for EmailHandler {
    fn get_job_id(&self) -> &str {
        "send_email"
    }

    async fn run(
        &self,
        job: &crate::capabilities::background::JobModel,
        _: Option<AppState>,
    ) -> Result<Option<String>, crate::capabilities::lib::common_error::CommonError> {
        let message: NotificationMailMessage =
            serde_json::from_str(&job.payload.clone().unwrap()).unwrap();
        logger::info(&format!(
            "[bg][notifications][handler] Sending email to {}",
            message.email
        ));
        let d = match message.data {
            Some(d) => d,
            None => serde_json::json!({}),
        };
        let value = crate::capabilities::mailer::Mailer::new();
        logger::info("[bg][notifications][handler] Mailer created");
        let result = value
            .send_email(
                &message.template,
                &message.subject,
                &message.name,
                &message.email,
                d,
            )
            .await;
        if let Err(e) = result {
            logger::error(&format!(
                "[bg][notifications][handler] Error sending email: {}",
                e
            ));
            return Err(crate::capabilities::lib::common_error::CommonError::from(
                e.to_string(),
            ));
        }
        logger::info(&format!(
            "[bg][notifications][handler] Email sent successfully to {}",
            message.email
        ));
        Ok(Some("Email sent successfully".to_owned()))
    }
}
