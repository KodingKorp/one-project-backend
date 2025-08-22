use sea_orm::ActiveModelTrait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::capabilities::lib::common_error::CommonError;

use super::entities::jobs;

pub use jobs::JobStatus;
pub use jobs::JobType;
pub use jobs::Model as JobModel;

use chrono::Utc;
use cron::Schedule;
use std::str::FromStr;

pub async fn get_active_jobs(
    db: &DatabaseConnection,
    queue: &str,
) -> Result<Vec<jobs::Model>, CommonError> {
    jobs::Entity::find()
        .filter(jobs::Column::Status.eq(jobs::JobStatus::Active))
        .filter(jobs::Column::Queue.eq(queue))
        .all(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))
}

pub async fn get_pending_jobs(
    db: &DatabaseConnection,
    job_id: &str,
    queue: &str,
) -> Result<Vec<jobs::Model>, CommonError> {
    jobs::Entity::find()
        .filter(jobs::Column::Status.eq(jobs::JobStatus::Active))
        .filter(jobs::Column::JobId.eq(job_id))
        .filter(jobs::Column::NextRunAt.lte(chrono::Utc::now().naive_utc()))
        .filter(jobs::Column::JobType.ne(jobs::JobType::Schedule))
        .filter(jobs::Column::Queue.eq(queue))
        .all(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))
}

pub async fn get_active_schedules(
    db: &DatabaseConnection,
    queue: &str,
) -> Result<Vec<jobs::Model>, CommonError> {
    jobs::Entity::find()
        .filter(jobs::Column::Status.eq(jobs::JobStatus::Active))
        .filter(jobs::Column::JobType.eq(jobs::JobType::Schedule))
        .filter(jobs::Column::Queue.eq(queue))
        .all(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))
}

pub async fn get_active_schedule_by_job_id(
    db: &DatabaseConnection,
    job_id: &str,
    queue: &str,
) -> Result<Option<jobs::Model>, CommonError> {
    jobs::Entity::find()
        .filter(jobs::Column::Status.eq(jobs::JobStatus::Active))
        .filter(jobs::Column::JobType.eq(jobs::JobType::Schedule))
        .filter(jobs::Column::JobId.eq(job_id))
        .filter(jobs::Column::Queue.eq(queue))
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))
}

pub async fn create_job(
    db: &DatabaseConnection,
    job_id: &str,
    job_type: jobs::JobType,
    queue: &str,
    payload: Option<String>,
    max_retries: Option<i32>,
    pattern: Option<String>,
    delay: Option<i32>,
    retry: Option<i32>,
    id: Option<i32>,
) -> Result<jobs::Model, CommonError> {
    let mut job = jobs::ActiveModel {
        job_id: Set(job_id.to_owned()),
        job_type: Set(job_type.clone()),
        queue: Set(queue.to_owned()),
        payload: Set(payload),
        pattern: Set(pattern.clone()),
        status: Set(jobs::JobStatus::Active),
        next_run_at: Set(Some(chrono::Utc::now().naive_utc())), // default immediate
        linked_job_id: Set(id),
        ..Default::default()
    };
    if let Some(max_retries) = max_retries {
        job.max_retries = Set(max_retries);
    }
    if let Some(pattern) = pattern {
        if job_type != jobs::JobType::Schedule {
            return Err(CommonError::from(
                "Pattern is only allowed for scheduled jobs".to_owned(),
            ));
        }
        let cron_schedule =
            Schedule::from_str(&pattern).map_err(|e| CommonError::from(e.to_string()))?;
        let next_run_at = cron_schedule
            .upcoming(Utc)
            .next()
            .ok_or_else(|| CommonError::from("Invalid cron pattern".to_owned()))?;
        job.next_run_at = Set(Some(next_run_at.naive_utc()));
    }

    if let Some(delay) = delay {
        if job_type != jobs::JobType::Delayed {
            return Err(CommonError::from(
                "Delay is only allowed for delayed jobs".to_owned(),
            ));
        }
        job.delay = Set(delay);
        job.next_run_at = Set(Some(
            chrono::Utc::now().naive_utc() + chrono::Duration::milliseconds(delay as i64),
        ));
    }

    if let Some(retry) = retry {
        if retry >= max_retries.unwrap_or(0) {
            return Err(CommonError::from(
                "Retry count should be less than max retries".to_owned(),
            ));
        }
        job.retries = Set(retry);
    }
    job.insert(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))
}

pub async fn get_job_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<jobs::Model>, CommonError> {
    jobs::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))
}

pub async fn update_job_status(
    db: &DatabaseConnection,
    job: &jobs::Model,
    status: jobs::JobStatus,
    output: Option<String>,
) -> Result<jobs::Model, CommonError> {
    let mut job: jobs::ActiveModel = job.clone().into();
    job.status = Set(status.clone());
    job.output = Set(output);
    match status {
        jobs::JobStatus::Running => {
            job.last_ran_at = Set(Some(chrono::Utc::now().naive_utc()));
        }
        jobs::JobStatus::Completed => {
            job.completed_at = Set(Some(chrono::Utc::now().naive_utc()));
        }
        jobs::JobStatus::Failed => {
            job.failed_at = Set(Some(chrono::Utc::now().naive_utc()));
        }
        _ => {}
    }

    let job = job
        .update(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    Ok(job)
}

pub async fn generate_immediate_job_from_schedule_job(
    db: &DatabaseConnection,
    id: i32,
) -> Result<JobModel, CommonError> {
    let scheduled_job = jobs::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))?;
    if scheduled_job.is_none() {
        return Err(CommonError::from("Job not found".to_owned()));
    }
    let scheduled_job = scheduled_job.unwrap();

    if scheduled_job.job_type != jobs::JobType::Schedule {
        return Err(CommonError::from("Job is not a scheduled job".to_owned()));
    }

    if scheduled_job.pattern.is_none() {
        return Err(CommonError::from(
            "Pattern is not set for scheduled job".to_owned(),
        ));
    }

    let pattern = scheduled_job.pattern.clone().unwrap();

    let cron_schedule =
        Schedule::from_str(&pattern).map_err(|e| CommonError::from(e.to_string()))?;
    let next_run_at = cron_schedule
        .upcoming(Utc)
        .next()
        .ok_or_else(|| CommonError::from("Invalid cron pattern".to_owned()))?;
    jobs::Entity::update(jobs::ActiveModel {
        id: Set(scheduled_job.id),
        next_run_at: Set(Some(next_run_at.naive_utc())),
        ..Default::default()
    })
    .exec(db)
    .await
    .map_err(|e| CommonError::from(e.to_string()))?;

    // Create new immediate job
    create_job(
        db,
        &scheduled_job.job_id,
        jobs::JobType::Immediate,
        &scheduled_job.queue,
        scheduled_job.payload,
        Some(scheduled_job.max_retries),
        None,
        None,
        None,
        Some(id),
    )
    .await
}

pub async fn update_job(
    db: &DatabaseConnection,
    id: i32,
    job_id: &str,
    job_type: jobs::JobType,
    payload: Option<String>,
    max_retries: Option<i32>,
    pattern: Option<String>,
    delay: Option<i32>,
    retry: Option<i32>,
    next_run_at: Option<chrono::NaiveDateTime>,
    status: jobs::JobStatus,
    output: Option<String>,
) -> Result<jobs::Model, CommonError> {
    let mut job = jobs::ActiveModel {
        id: Set(id),
        job_id: Set(job_id.to_owned()),
        job_type: Set(job_type.clone()),
        payload: Set(payload),
        pattern: Set(pattern.clone()),
        status: Set(status.clone()),
        next_run_at: Set(next_run_at),
        ..Default::default()
    };

    if let Some(max_retries) = max_retries {
        job.max_retries = Set(max_retries);
    }

    if let Some(pattern) = pattern {
        if job_type != jobs::JobType::Schedule {
            return Err(CommonError::from(
                "Pattern is only allowed for scheduled jobs".to_owned(),
            ));
        }
        let cron_schedule =
            Schedule::from_str(&pattern).map_err(|e| CommonError::from(e.to_string()))?;
        let next_run_at = cron_schedule
            .upcoming(Utc)
            .next()
            .ok_or_else(|| CommonError::from("Invalid cron pattern".to_owned()))?;
        job.next_run_at = Set(Some(next_run_at.naive_utc()));
    }

    if let Some(delay) = delay {
        if delay != 0 {
            if job_type != jobs::JobType::Delayed {
                return Err(CommonError::from(
                    "Delay is only allowed for delayed jobs".to_owned(),
                ));
            }
            job.delay = Set(delay);
            job.next_run_at = Set(Some(
                chrono::Utc::now().naive_utc() + chrono::Duration::milliseconds(delay as i64),
            ));
        }
    }

    if let Some(retry) = retry {
        if retry > max_retries.unwrap_or(0) {
            return Err(CommonError::from(
                "Retry count should be less than max retries".to_owned(),
            ));
        }
        job.retries = Set(retry);
    }

    job.output = Set(output);

    job.update(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))
}
