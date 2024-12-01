use sea_orm::DatabaseConnection;
use tokio::{
    sync::mpsc::Receiver,
    time,
};

use crate::capabilities::{lib::common_error::CommonError, logger};

use super::{jobs, queues::QueueMessage};
use crate::bootstrap::AppState;
#[async_trait::async_trait]
pub trait JobHandler: Send + Sync {
    fn get_job_id(&self) -> &str;
    /// Run the job
    async fn run(
        &self,
        job: &super::JobModel,
        app_state: Option<AppState>,
    ) -> Result<Option<String>, CommonError>;
}

pub struct JobScheduler {
    pub name: String,
    pub poll_interval: u64,
}

impl JobScheduler {
    pub fn new(poll_interval: u64, name: String) -> Self {
        Self {
            name,
            poll_interval,
        }
    }

    pub async fn run(
        &mut self,
        db: &DatabaseConnection,
    ) -> Result<Receiver<QueueMessage>, CommonError> {
        logger::info(&format!(
            "[bg][scheduler][{}] Starting scheduler",
            &self.name
        ));

        let (scheduler_channel_tx, scheduler_channel_rx) =
            tokio::sync::mpsc::channel::<QueueMessage>(100);
        let db = db.clone();
        let mut interval = time::interval(time::Duration::from_secs(self.poll_interval));
        logger::info(&format!("[bg][scheduler][{}] Spawning Runner", &self.name));
        let name = self.name.clone();
        let mut last_poll = chrono::Utc::now();
        let mut last_complete = chrono::Utc::now();
        tokio::task::spawn(async move {
            loop {
                logger::info(&format!(
                    "[bg][scheduler][{}] Time between polls: {}ms",
                    &name,
                    last_complete
                        .signed_duration_since(last_poll)
                        .num_milliseconds()
                ));
                last_poll = chrono::Utc::now();

                logger::info(&format!(
                    "[bg][scheduler][{}] Polling for jobs: {}",
                    &name,
                    last_poll.format("%d/%m/%Y %H:%M:%S")
                ));

                let db_jobs = jobs::get_active_schedules(&db, &name).await;
                logger::debug(&format!(
                    "[bg][scheduler][{}] Found {} jobs",
                    &name,
                    db_jobs.as_ref().unwrap().len()
                ));
                if let Ok(db_jobs) = db_jobs {
                    for job in db_jobs {
                        logger::debug(&format!(
                            "[bg][scheduler][{}] Checking job: {} Next Run At: {:?}",
                            &name,
                            &job.id,
                            &job.next_run_at
                        ));
                        if let Some(next_run_time) = job.next_run_at
                        {
                            if next_run_time <= last_poll.naive_utc() {
                                logger::info(&format!(
                                    "[bg][scheduler][{}] Triggering job: {}",
                                    &name, job.id
                                ));
                                scheduler_channel_tx
                                    .send(QueueMessage::TriggerSchedule(job.id))
                                    .await
                                    .unwrap();
                            }
                        }
                    }
                }
                last_complete = chrono::Utc::now();
                logger::info(&format!(
                    "[bg][scheduler][{}] Completed Polling for jobs: {}, Duration: {}ms",
                    &name,
                    last_complete.format("%d/%m/%Y %H:%M:%S"),
                    last_complete
                        .signed_duration_since(last_poll)
                        .num_milliseconds()
                ));
                interval.tick().await;
            }
        });
        Ok(scheduler_channel_rx)
    }
}
