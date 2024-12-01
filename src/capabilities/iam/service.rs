use std::sync::Arc;

use crate::capabilities::{background::{orchestrator::{CreateJob, CreateQueue, CreateSchedule}, JobHandler}, lib::service_trait::Service};
use poem_openapi::OpenApi;

use super::{controllers::authentication, job_handlers::{abandoned_cart_cron_handler, register_side_effect_handler::RegisterSideEffectHandler}};

#[derive(Clone)]
pub struct IAMService;
#[async_trait::async_trait]
impl Service for IAMService {
    fn register_routes() -> Option<impl OpenApi> {
        Some(authentication::Api)
    }

    async fn register_background(
        mut orchestrator: crate::capabilities::background::orchestrator::BackgroundOrchestrator,
    ) -> crate::capabilities::background::orchestrator::BackgroundOrchestrator {
        let register_handler = RegisterSideEffectHandler::new();
        let abandoned_cart_cron_handler = abandoned_cart_cron_handler::AbandonedCartHandler::new();
        orchestrator.register_queue(CreateQueue {
            name: "iam".to_owned(),
        }).await;
        orchestrator.register_job(CreateJob {
            name: register_handler.get_job_id().to_owned(),
            handler: Arc::new(register_handler),
            queue: "iam".to_owned(),
        }).await;
        orchestrator.register_schedule(CreateSchedule {
            job: abandoned_cart_cron_handler.get_job_id().to_owned(),
            queue: "iam".to_owned(),
            schedule: abandoned_cart_cron_handler.get_pattern(),
        }).await;
        orchestrator.register_job(CreateJob {
            name: abandoned_cart_cron_handler.get_job_id().to_owned(),
            handler: Arc::new(abandoned_cart_cron_handler),
            queue: "iam".to_owned(),
        }).await;
        
        orchestrator
    }
}
