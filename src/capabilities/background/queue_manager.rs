use std::{collections::HashMap, sync::Arc};

use super::{queues::Queue, JobHandler};
use crate::{bootstrap::AppState, capabilities::logger};
pub struct QueueManager {
    pub queues: HashMap<String, Queue>,
    pub app_state: Option<AppState>,
}

impl QueueManager {
    pub fn new(app_state: Option<AppState>) -> Self {
        QueueManager {
            queues: HashMap::new(),
            app_state,
        }
    }
    /// Create a new queue
    pub async fn create_queue(&mut self, queue_name: String) {
        let queue = Queue::new(queue_name.clone(), self.app_state.clone()).await;
        self.queues.insert(queue_name, queue);
    }

    /// Get queue
    pub fn get_queue(&self, queue_name: &str) -> Option<&Queue> {
        self.queues.get(queue_name)
    }

    /// set job handler for the queue
    pub async fn set_job_handler(
        &mut self,
        queue_name: &str,
        job_name: &str,
        job_handler: Arc<dyn JobHandler>,
    ) {
        if let Some(queue) = self.queues.get_mut(queue_name) {
            queue.register_handler(job_name, job_handler).await;
        }
    }

    pub async fn upsert_schedule(&mut self, queue_name: &str, job_name: &str, pattern: String) {
        if let Some(queue) = self.queues.get_mut(queue_name) {
            let _ = queue.upsert_schedule(job_name, &pattern, None, None).await;
        }
    }

    /// trigger job
    pub async fn trigger_job(&mut self, queue_name: &str, job_name: &str, payload: Option<String>) {
        if let Some(queue) = self.queues.get_mut(queue_name) {
            let _ = queue
                .create_immediate_job(job_name, payload, None, None, None)
                .await;
        }
    }

    /// Start all queues
    pub async fn start(&mut self) {
        // start the queue manager
        let iterator = self.queues.iter_mut();
        for (name, queue) in iterator {
            logger::info(&format!("[bg][queue_manager] Starting queue {}", name));
            let _ = queue.start().await;
            logger::info(&format!("[bg][queue_manager] Started queue {}", name));
        }
        logger::info("[bg][queue_manager] Started all queues");
    }
}
