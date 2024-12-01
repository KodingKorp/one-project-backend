use std::sync::Arc;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};

use crate::{bootstrap::AppState, capabilities::logger};

use super::{queue_manager, JobHandler};
#[derive(Clone)]
pub struct CreateQueue {
    pub name: String,
}
#[derive(Clone)]
pub struct CreateJob {
    pub queue: String,
    pub name: String,
    pub handler: Arc<dyn JobHandler>,
}
#[derive(Clone)]
pub struct CreateSchedule {
    pub schedule: cron::Schedule,
    pub queue: String,
    pub job: String,
}
#[derive(Clone)]
pub struct RunJob {
    pub queue: String,
    pub name: String,
    pub job_type: super::JobType,
    pub payload: Option<String>,
    pub max_retries: Option<i32>,
    pub delay: Option<i32>,
}

#[derive(Clone)]
enum OrchestratorMessage {
    Ping,
    Pong,
    Start,
    RegisterQueue(CreateQueue),
    RegisterJob(Box<CreateJob>),
    RegisterSchedule(CreateSchedule),
    RunJob(RunJob),
    Acknowledge,
}

#[derive(Clone)]
pub struct BackgroundOrchestrator {
    worker_tx: Sender<OrchestratorMessage>,
    orchestrator_rx: Arc<Mutex<Receiver<OrchestratorMessage>>>,
}

impl BackgroundOrchestrator {
    pub fn new(app_state: Option<AppState>) -> Self {
        // Create a new channel for the orchestrator
        let (worker_tx, mut worker_rx) = tokio::sync::mpsc::channel::<OrchestratorMessage>(100);
        let (orchestrator_tx, orchestrator_rx) =
            tokio::sync::mpsc::channel::<OrchestratorMessage>(100);
        let obj = BackgroundOrchestrator {
            worker_tx,
            orchestrator_rx: Arc::new(Mutex::new(orchestrator_rx)),
        };
        // Create a thread
        tokio::spawn(async move {
            let mut queue_manager = queue_manager::QueueManager::new(app_state);
            while let Some(msg) = worker_rx.recv().await {
                match msg {
                    OrchestratorMessage::Start => {
                        logger::debug(&format!(
                            "[bg][orchestrator] Starting queues",
                        ));
                        queue_manager.start().await;
                        let _ = orchestrator_tx.send(OrchestratorMessage::Acknowledge).await;
                    }
                    OrchestratorMessage::RegisterQueue(queue) => {
                        logger::debug(&format!(
                            "[bg][orchestrator] Registering queue {}",
                            queue.name
                        ));
                        queue_manager.create_queue(queue.name).await;
                        let _ = orchestrator_tx.send(OrchestratorMessage::Acknowledge).await;
                    }
                    OrchestratorMessage::RegisterJob(job) => {
                        logger::debug(&format!("[bg][orchestrator] Registering job {}", job.name));
                        queue_manager.set_job_handler(&job.queue, &job.name, job.handler).await;
                        let _ = orchestrator_tx.send(OrchestratorMessage::Acknowledge).await;
                    }
                    OrchestratorMessage::RegisterSchedule(schedule) => {
                        logger::debug(&format!(
                            "[bg][orchestrator] Registering schedule {}",
                            schedule.queue
                        ));
                        queue_manager.upsert_schedule(&schedule.queue, &schedule.job, schedule.schedule.to_string()).await;
                        let _ = orchestrator_tx.send(OrchestratorMessage::Acknowledge).await;
                    }
                    OrchestratorMessage::RunJob(job) => {
                        logger::debug(&format!("[bg][orchestrator] Running job {}", job.name));
                        queue_manager.trigger_job(&job.queue, &job.name, job.payload).await;
                        let _ = orchestrator_tx.send(OrchestratorMessage::Acknowledge).await;
                    }
                    OrchestratorMessage::Ping => {
                        let _ = orchestrator_tx.send(OrchestratorMessage::Pong).await;
                    }
                    _ => (),
                }
            }
            
        });
        obj
    }
    /// Check if the orchestrator is connected
    pub async fn health(&self) -> bool {
        let result = self.worker_tx.send(OrchestratorMessage::Ping).await;
        if result.is_err() {
            return false;
        } else if let Some(OrchestratorMessage::Pong) =
            self.orchestrator_rx.lock().await.recv().await
        {
            return true;
        }
        false
    }
    /// Register a new queue
    pub async fn register_queue(&mut self, queue: CreateQueue) -> bool {
        let result = self
            .worker_tx
            .send(OrchestratorMessage::RegisterQueue(queue))
            .await;
        if result.is_err() {
            return false;
        } else if let Some(OrchestratorMessage::Acknowledge) =
            self.orchestrator_rx.lock().await.recv().await
        {
            return true;
        }
        false
    }

    /// Register a new job handler
    pub async fn register_job(&mut self, job: CreateJob) -> bool {
        let result = self
            .worker_tx
            .send(OrchestratorMessage::RegisterJob(Box::new(job.clone())))
            .await;
        if result.is_err() {
            return false;
        } else if let Some(OrchestratorMessage::Acknowledge) =
            self.orchestrator_rx.lock().await.recv().await
        {
            return true;
        }
        false
    }

    /// Register a new schedule
    pub async fn register_schedule(&mut self, schedule: CreateSchedule) -> bool {
        let result = self
            .worker_tx
            .send(OrchestratorMessage::RegisterSchedule(schedule))
            .await;
        if result.is_err() {
            return false;
        } else if let Some(OrchestratorMessage::Acknowledge) =
            self.orchestrator_rx.lock().await.recv().await
        {
            return true;
        }
        false
    }

    /// Run a job
    pub async fn run_job(&mut self, job: RunJob) -> bool {
        let result = self.worker_tx.send(OrchestratorMessage::RunJob(job)).await;
        if result.is_err() {
            return false;
        } else if let Some(OrchestratorMessage::Acknowledge) =
            self.orchestrator_rx.lock().await.recv().await
        {
            return true;
        }
        false
    }

    pub async fn start(&mut self) -> bool {
        let result = self.worker_tx.send(OrchestratorMessage::Start).await;
        if result.is_err() {
            return false;
        } else if let Some(OrchestratorMessage::Acknowledge) =
            self.orchestrator_rx.lock().await.recv().await
        {
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use crate::capabilities::lib::common_error::CommonError;

    use super::*;

    struct TestWorker;
    #[async_trait::async_trait]
    impl JobHandler for TestWorker {
        fn get_job_id(&self) -> &str {
            "test"
        }
        async fn run(
            &self,
            _job: &super::super::JobModel,
            _app_state: Option<AppState>,
        ) -> Result<Option<String>, CommonError> {
            Ok(None)
        }
    }

    /// Test the background orchestrator instance
    #[tracing_test::traced_test]
    #[tokio::test()]
    async fn test_background_orchestrator_instance_correctly() {
        let app_state = None;
        let orchestrator = BackgroundOrchestrator::new(app_state);
        assert!(orchestrator.health().await);
    }

    /// Test the background orchestrator register queue
    #[tracing_test::traced_test]
    #[tokio::test()]
    async fn test_background_orchestrator_register_queue() {
        dotenvy::dotenv().ok();
        let app_state = None;
        let mut orchestrator = BackgroundOrchestrator::new(app_state);
        let queue = CreateQueue {
            name: "test".to_string(),
        };
        assert!(orchestrator.register_queue(queue).await);
    }
    /// Test the background orchestrator register job
    #[tracing_test::traced_test]
    #[tokio::test()]
    async fn test_background_orchestrator_register_job() {
        dotenvy::dotenv().ok();
        let app_state = None;
        let mut orchestrator = BackgroundOrchestrator::new(app_state);
        let queue = CreateQueue {
            name: "test".to_string(),
        };
        assert!(orchestrator.register_queue(queue).await);
        let job = CreateJob {
            name: "test".to_string(),
            queue: "test".to_string(),
            handler: Arc::new(TestWorker {}),
        };
        assert!(orchestrator.register_job(job).await);
    }

    /// Test the background orchestrator register schedule
    #[tracing_test::traced_test]
    #[tokio::test()]
    async fn test_background_orchestrator_register_schedule() {
        let app_state = None;
        let mut orchestrator = BackgroundOrchestrator::new(app_state);
        let schedule = CreateSchedule {
            schedule: cron::Schedule::from_str("0 0 * * * *").unwrap(),
            queue: "test".to_string(),
            job: "test".to_string(),
        };
        assert!(orchestrator.register_schedule(schedule).await);
    }

    /// Test the background orchestrator run job
    #[tracing_test::traced_test]
    #[tokio::test()]
    async fn test_background_orchestrator_run_job() {
        let app_state = None;
        let mut orchestrator = BackgroundOrchestrator::new(app_state);
        let job = RunJob {
            queue: "test".to_string(),
            name: "test".to_string(),
            job_type: super::super::JobType::Immediate,
            payload: None,
            max_retries: None,
            delay: None,
        };
        assert!(orchestrator.run_job(job).await);
    }

    /// Test the background orchestrator start
    #[tracing_test::traced_test]
    #[tokio::test()]
    async fn test_background_orchestrator_start() {
        let app_state = None;
        let mut orchestrator = BackgroundOrchestrator::new(app_state);
        assert!(orchestrator.start().await);
    }
}
