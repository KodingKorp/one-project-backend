use super::{
    job_handler::{JobHandler, JobScheduler},
    jobs,
    workers::Worker,
};
use crate::{
    bootstrap::AppState,
    capabilities::{config, database, lib::common_error::CommonError, logger},
};
use sea_orm::DatabaseConnection;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::Arc,
};
use tokio::sync::Mutex;
pub type HandlerMap = Arc<Mutex<HashMap<String, Arc<dyn JobHandler>>>>;

pub enum QueueMessage {
    TriggerMany(Vec<(String, i32)>),
    TriggerOne(String, i32),
    TriggerSchedule(i32),
    Job(i32),
}
pub struct Queue {
    db: DatabaseConnection,
    pub name: String,
    job_ids: HashSet<String>,
    handlers: HandlerMap,
    worker_channel_tx: Option<tokio::sync::mpsc::Sender<QueueMessage>>,
    scheduler_channel_rx: Option<tokio::sync::mpsc::Receiver<QueueMessage>>,
    app_state: Option<AppState>,
}

impl Queue {
    pub async fn new(name: String, app_state: Option<AppState>) -> Self {
        logger::info(&format!("[bg][queue][{}] Creating queue", &name));
        let db = database::make_pg_db_connection().await;
        Queue {
            db,
            name,
            handlers: Arc::new(Mutex::new(HashMap::new())),
            job_ids: HashSet::new(),
            worker_channel_tx: None,
            scheduler_channel_rx: None,
            app_state,
        }
    }

    async fn check_pending_jobs(&mut self) -> Result<(), CommonError> {
        logger::debug(&format!(
            "[bg][queue][{}] Checking pending jobs",
            &self.name
        ));
        for job_id in &self.job_ids {
            let jobs = jobs::get_pending_jobs(&self.db, job_id, &self.name)
                .await
                .unwrap();
            logger::debug(&format!(
                "[bg][queue][{}] Found {} pending jobs for {}",
                &self.name,
                jobs.len(),
                job_id
            ));
            for job in jobs {
                logger::info(&format!(
                    "[bg][queue][{}] Processing job {}",
                    &self.name, job.id
                ));

                let _ = self
                    .worker_channel_tx
                    .as_mut()
                    .ok_or_else(|| CommonError::from("TX not set".to_owned()))?
                    .send(QueueMessage::Job(job.id))
                    .await
                    .map_err(|e| {
                        logger::error(&format!(
                            "[bg][queue][{}] Error in creating job {} : {}",
                            &self.name, job.job_id, e
                        ));
                        CommonError::from("Failed to publish job".to_owned())
                    });
            }
        }
        Ok(())
    }

    pub async fn register_handler(&mut self, job_id: &str, handler: Arc<dyn JobHandler>) {
        self.job_ids.insert(job_id.to_owned());
        logger::info(&format!(
            "[bg][queue][{}] Registering job {}",
            &self.name, job_id
        ));
        let mut handlers = self.handlers.lock().await;
        handlers.insert(job_id.to_owned(), handler);
    }

    /// Create a job to run without delay
    pub async fn create_immediate_job(
        &mut self,
        job_id: &str,
        payload: Option<String>,
        max_retries: Option<i32>,
        retry: Option<i32>,
        id: Option<i32>,
    ) -> Result<i32, CommonError> {
        logger::info(&format!(
            "[bg][queue][{}] Creating immediate job: {}",
            &self.name, job_id
        ));
        let create_job_params = jobs::CreateJobParams {
            job_id,
            job_type: jobs::JobType::Immediate,
            queue: &self.name,
            payload,
            max_retries,
            pattern: None,
            delay: None,
            retry,
            id,
        };
        let job = jobs::create_job(
            &self.db,
            create_job_params,
        )
        .await?;
        logger::info(&format!(
            "[bg][queue][{}] Created immediate job: {}",
            &self.name, job_id
        ));
        self
            .worker_channel_tx
            .as_mut()
            .ok_or_else(|| CommonError::from("TX not set".to_owned()))?
            .send(QueueMessage::Job(job.id))
            .await
            .map_err(|e| {
                logger::error(&format!(
                    "[bg][queue][{}] Error in creating job {} : {}",
                    &self.name, job.job_id, e
                ));
                CommonError::from("Failed to publish job".to_owned())
            })?;
        Ok(job.id)
    }
    /// Create a job to run after a delay in milliseconds
    pub async fn create_delayed_job(
        &self,
        job_id: &str,
        delay: i32,
        payload: Option<String>,
        max_retries: Option<i32>,
        id: Option<i32>,
    ) -> Result<i32, CommonError> {
        logger::info(&format!(
            "[bg][queue][{}] Creating delayed job: {}, with delay: {}",
            &self.name, job_id, delay
        ));

        let create_job_params = jobs::CreateJobParams {
            job_id,
            job_type: jobs::JobType::Delayed,
            queue: &self.name,
            payload,
            max_retries,
            pattern: None,
            delay: Some(delay),
            retry: None,
            id,
        };

        let job = jobs::create_job(
            &self.db,
            create_job_params,
        )
        .await?;
        logger::info(&format!(
            "[bg][queue][{}] Created delayed job: {}, with delay: {}",
            &self.name, job_id, delay
        ));
        Ok(job.id)
    }

    /// Create a repeatable job to run at a scheduled time
    pub async fn create_scheduled_job(
        &self,
        job_id: &str,
        pattern: &str,
        payload: Option<String>,
        max_retries: Option<i32>,
    ) -> Result<i32, CommonError> {
        logger::info(&format!(
            "[bg][queue][{}] Creating scheduled job: {}, with pattern: {}",
            &self.name, job_id, pattern
        ));

        let create_job_params = jobs::CreateJobParams {
            job_id,
            job_type: jobs::JobType::Schedule,
            queue: &self.name,
            payload,
            max_retries,
            pattern: Some(pattern.to_owned()),
            delay: None,
            retry: None,
            id: None,
        };

        let job = jobs::create_job(
            &self.db,
            create_job_params,
        )
        .await?;
        logger::info(&format!(
            "[bg][queue][{}] Created scheduled job: {}, with pattern: {}",
            &self.name, job_id, pattern
        ));
        Ok(job.id)
    }

    /// Generate an immediate job from another job with retry functionality
    pub async fn generate_immediate_job_from_existing_job(
        &mut self,
        id: i32,
        retry: bool,
    ) -> Result<i32, CommonError> {
        let job = jobs::get_job_by_id(&self.db, id)
            .await?
            .ok_or(CommonError::from("Job not found".to_owned()))?;
        let mut retry_count = job.retries;
        if retry {
            retry_count += 1;
        }
        let new_job_id = self
            .create_immediate_job(
                &job.job_id,
                job.payload.clone(),
                Some(job.max_retries),
                Some(retry_count),
                Some(id),
            )
            .await?;
        Ok(new_job_id)
    }

    /// Generate an immediate job from another job with retry functionality
    pub async fn generate_delayed_job_from_schedule_job(
        &mut self,
        id: i32,
    ) -> Result<i32, CommonError> {
        let job = jobs::get_job_by_id(&self.db, id)
            .await?
            .ok_or(CommonError::from("Job not found".to_owned()))?;
        if job.job_type != jobs::JobType::Schedule {
            return Err(CommonError::from("Job is not a scheduled job".to_owned()));
        }

        if job.pattern.is_none() {
            return Err(CommonError::from("Job pattern is missing".to_owned()));
        }

        let next_run = cron::Schedule::from_str(&job.pattern.unwrap())
            .map_err(|e| CommonError::from(e.to_string()))?
            .upcoming(chrono::Utc)
            .next()
            .ok_or(CommonError::from("Invalid cron pattern".to_owned()))?;

        let delay: i32 = next_run
            .signed_duration_since(chrono::Utc::now())
            .num_milliseconds()
            .max(i32::MAX as i64)
            .min(i32::MIN as i64) as i32;
        let new_job_id = self
            .create_delayed_job(
                &job.job_id,
                delay,
                job.payload.clone(),
                Some(job.max_retries),
                Some(id),
            )
            .await?;
        Ok(new_job_id)
    }

    pub async fn upsert_schedule(
        &self,
        job_id: &str,
        pattern: &str,
        payload: Option<String>,
        max_retries: Option<i32>,
    ) -> Result<i32, CommonError> {
        let existing_job =
            jobs::get_active_schedule_by_job_id(&self.db, job_id, &self.name).await?;
        if let Some(existing_job) = existing_job {
            logger::info(&format!(
                "[bg][queue][{}] Updating schedule job: {}",
                &self.name, job_id
            ));

            let update_job_params = jobs::UpdateJobParams {
                id: existing_job.id,
                job_id,
                job_type: jobs::JobType::Schedule,
                payload: payload.clone(),
                max_retries,
                pattern: Some(pattern.to_owned()),
                delay: Some(existing_job.delay),
                retry: Some(existing_job.retries),
                next_run_at: existing_job.next_run_at,
                status: existing_job.status.clone(),
                output: existing_job.output.clone(),
            };

            let updated_job = jobs::update_job(
                &self.db,
                update_job_params,
            )
            .await?;
            Ok(updated_job.id)
        } else {
            return self
                .create_scheduled_job(job_id, pattern, payload, max_retries)
                .await;
        }
    }
    async fn start_scheduler(&mut self) {
        let mut poll_interval: u64 = config::get_env("SCHEDULER_POLL_INTERVAL");

        if poll_interval == 0 {
            poll_interval = 5; // default poll 5 seconds
        }

        logger::info(&format!("[bg][queue][{}] Starting Scheduler", &self.name));

        let mut scheduler_channel_rx = JobScheduler::new(poll_interval, self.name.clone())
            .run(&self.db)
            .await
            .unwrap();
        let worker_channel_tx = self.worker_channel_tx.clone().unwrap();
        let name = self.name.clone();
        let db = self.db.clone();
        tokio::task::spawn(async move {
            while let Some(QueueMessage::TriggerSchedule(id)) = scheduler_channel_rx.recv().await {
                logger::info(&format!(
                    "[bg][queue][{}] Triggering schedule {}",
                    &name, id
                ));
                let new_job = jobs::generate_immediate_job_from_schedule_job(&db, id)
                    .await
                    .unwrap();
                let _ = worker_channel_tx
                    .send(QueueMessage::Job(new_job.id))
                    .await
                    .map_err(|e| {
                        logger::error(&format!(
                            "[bg][queue][{}] Error in creating job {} : {}",
                            &name, id, e
                        ));
                        CommonError::from("Failed to publish job".to_owned())
                    });
            }
        });
    }
    pub async fn start(&mut self) {
        logger::info(&format!("[bg][queue][{}] Starting worker", &self.name));

        self.worker_channel_tx = Some(
            Worker::start_worker(
                self.name.clone(),
                &self.db,
                self.handlers.clone(),
                self.app_state.clone(),
            )
            .await,
        );
        logger::info(&format!("[bg][queue][{}] Started worker", &self.name));
        let _ = self.check_pending_jobs().await;

        self.start_scheduler().await;
    }
}
