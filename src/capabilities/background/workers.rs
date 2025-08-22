use crate::{
    bootstrap::AppState,
    capabilities::{lib::common_error::CommonError, logger},
};
use sea_orm::DatabaseConnection;
use tokio::sync::mpsc::Sender;

use super::{
    jobs::{self, update_job_status},
    queues::{HandlerMap, QueueMessage},
};
pub struct Worker;

impl Worker {
    pub async fn start_worker(
        name: String,
        db: &DatabaseConnection,
        handlers: HandlerMap,
        app_state: Option<AppState>,
    ) -> Sender<QueueMessage> {
        let name = name.clone();
        let db = db.clone();
        let handlers = handlers.clone();
        let (worker_channel_tx, mut worker_channel_rx) =
            tokio::sync::mpsc::channel::<QueueMessage>(100);
        tokio::spawn(async move {
            while let Some(QueueMessage::Job(id)) = worker_channel_rx.recv().await {
                logger::info(&format!(
                    "[bg][worker][{}] Processing job: {}",
                    name.clone(),
                    id
                ));
                let handlers = handlers.clone();
                let db = db.clone();
                let worker_name = name.clone();
                let app_state = app_state.clone();
                tokio::spawn(async move {
                    let job = jobs::get_job_by_id(&db, id)
                        .await
                        .unwrap()
                        .ok_or_else(|| CommonError::from("Job not found".to_owned()));
                    if let Err(e) = job {
                        logger::error(&format!(
                            "[bg][worker][{}] Error in starting job({}): {}",
                            &worker_name, &id, e
                        ));
                        return;
                    }
                    let job = job.unwrap();
                    let map = handlers.lock().await;
                    let handler_option = map.get(&job.job_id);
                    if handler_option.is_none() {
                        logger::error(&format!(
                            "[bg][worker][{}] Error in starting job: {}({}): Handler not found",
                            &worker_name, &job.job_id, &id
                        ));
                        return;
                    }
                    let handler = handler_option.unwrap();
                    logger::info(&format!(
                        "[bg][worker][{}] Running job: {}({})",
                        worker_name.clone(),
                        handler.get_job_id(),
                        &job.id
                    ));
                    let _ = update_job_status(&db, &job, jobs::JobStatus::Running, None)
                        .await
                        .map_err(|e| {
                            logger::error(&format!("{}", e));
                        });
                    let result = handler.run(&job, app_state.clone()).await;
                    match result {
                        Ok(output) => {
                            logger::info(&format!(
                                "[bg][worker][{}] Completed job: {}({})",
                                worker_name.clone(),
                                handler.get_job_id(),
                                &job.id
                            ));
                            let _ =
                                update_job_status(&db, &job, jobs::JobStatus::Completed, output)
                                    .await
                                    .map_err(|e| {
                                        logger::error(&format!("{}", e));
                                    });
                        }
                        Err(e) => {
                            logger::error(&format!(
                                "[bg][worker][{}] Error job {}({}): {}",
                                worker_name.clone(),
                                handler.get_job_id(),
                                &job.id,
                                e
                            ));
                            let _ = update_job_status(
                                &db,
                                &job,
                                jobs::JobStatus::Failed,
                                Some(e.to_string()),
                            )
                            .await
                            .map_err(|e| {
                                logger::error(&format!(
                                    "[bg][worker][{}] Error job {}({}): {}",
                                    worker_name.clone(),
                                    handler.get_job_id(),
                                    &job.id,
                                    e
                                ));
                            });
                        }
                    }
                });
            }
        });
        return worker_channel_tx;
    }
}
