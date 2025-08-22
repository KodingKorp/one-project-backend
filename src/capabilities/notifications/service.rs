use std::sync::Arc;

use crate::capabilities::{background, lib::service_trait::Service, logger, mailer::Mailer};

use self::background::orchestrator::{BackgroundOrchestrator, CreateJob, CreateQueue, RunJob};

use super::{email_handler, objects::NotificationMailMessage};

pub struct NotificationService;
impl NotificationService {
    pub async fn send_mail(
        template: &str,
        subject: &str,
        name: &str,
        email: &str,
        data: Option<serde_json::Value>,
        mut orchestrator: BackgroundOrchestrator,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message = NotificationMailMessage {
            data,
            template: template.to_owned(),
            subject: subject.to_owned(),
            name: name.to_owned(),
            email: email.to_owned(),
        };
        let message = serde_json::to_string(&message)?;
        orchestrator
            .run_job(RunJob {
                queue: "notification".to_owned(), // match orchestrator.get_queue_by_name("notification") {
                delay: None,
                job_type: background::JobType::Immediate,
                payload: Some(message),
                max_retries: None,
                name: "send_email".to_owned(),
            })
            .await;
        Ok(())
    }

    pub async fn check_mailer_connection() -> bool {
        Mailer::new().check_connection().await
    }
}

#[async_trait::async_trait]
impl Service for NotificationService {
    async fn register_background(
        mut orchestrator: background::orchestrator::BackgroundOrchestrator,
    ) -> background::orchestrator::BackgroundOrchestrator {
        let email_handler = email_handler::EmailHandler::new();
        logger::info("Email Handler Created");
        orchestrator
            .register_queue(CreateQueue {
                name: "notification".to_owned(),
            })
            .await;
        logger::info("Notification Queue Registered");
        orchestrator
            .register_job(CreateJob {
                name: "send_email".to_owned(),
                queue: "notification".to_owned(),
                handler: Arc::new(email_handler),
            })
            .await;
        logger::info("Notification Job Registered");
        return orchestrator;
    }
}
