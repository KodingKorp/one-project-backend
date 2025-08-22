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

pub struct CreateJobParams<'a> {
    pub job_id: &'a str,
    pub job_type: jobs::JobType,
    pub queue: &'a str,
    pub payload: Option<String>,
    pub max_retries: Option<i32>,
    pub pattern: Option<String>,
    pub delay: Option<i32>,
    pub retry: Option<i32>,
    pub id: Option<i32>,
}

pub struct UpdateJobParams<'a> {
    pub id: i32,
    pub job_id: &'a str,
    pub job_type: jobs::JobType,
    pub payload: Option<String>,
    pub max_retries: Option<i32>,
    pub pattern: Option<String>,
    pub delay: Option<i32>,
    pub retry: Option<i32>,
    pub next_run_at: Option<chrono::NaiveDateTime>,
    pub status: jobs::JobStatus,
    pub output: Option<String>,
}


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
    params: CreateJobParams<'_>,
) -> Result<jobs::Model, CommonError> {
    let mut job = jobs::ActiveModel {
        job_id: Set(params.job_id.to_owned()),
        job_type: Set(params.job_type.clone()),
        queue: Set(params.queue.to_owned()),
        payload: Set(params.payload.clone()),
        pattern: Set(params.pattern.clone()),
        status: Set(jobs::JobStatus::Active),
        next_run_at: Set(Some(chrono::Utc::now().naive_utc())), // default immediate
        linked_job_id: Set(params.id),
        ..Default::default()
    };
    if let Some(max_retries) = params.max_retries {
        job.max_retries = Set(max_retries);
    }
    if let Some(pattern) = params.pattern {
        if params.job_type != jobs::JobType::Schedule {
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

    if let Some(delay) = params.delay {
        if params.job_type != jobs::JobType::Delayed {
            return Err(CommonError::from(
                "Delay is only allowed for delayed jobs".to_owned(),
            ));
        }
        job.delay = Set(delay);
        job.next_run_at = Set(Some(
            chrono::Utc::now().naive_utc() + chrono::Duration::milliseconds(delay as i64),
        ));
    }

    if let Some(retry) = params.retry {
        if retry >= params.max_retries.unwrap_or(0) {
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

    let create_job_params = CreateJobParams {
        job_id: &scheduled_job.job_id,
        job_type: jobs::JobType::Immediate,
        queue: &scheduled_job.queue,
        payload: scheduled_job.payload,
        max_retries: Some(scheduled_job.max_retries),
        pattern: None,
        delay: None,
        retry: None,
        id: Some(id),
    };
    // Create new immediate job
    create_job(
        db,
        create_job_params,
    )
    .await
}

pub async fn update_job(
    db: &DatabaseConnection,
    params: UpdateJobParams<'_>,
) -> Result<jobs::Model, CommonError> {
    let mut job = jobs::ActiveModel {
        id: Set(params.id),
        job_id: Set(params.job_id.to_owned()),
        job_type: Set(params.job_type.clone()),
        payload: Set(params.payload.clone()),
        pattern: Set(params.pattern.clone()),
        status: Set(params.status.clone()),
        next_run_at: Set(params.next_run_at),
        ..Default::default()
    };

    if let Some(max_retries) = params.max_retries {
        job.max_retries = Set(max_retries);
    }

    if let Some(pattern) = params.pattern {
        if params.job_type != jobs::JobType::Schedule {
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

    if let Some(delay) = params.delay {
        if delay != 0 {
            if params.job_type != jobs::JobType::Delayed {
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

    if let Some(retry) = params.retry {
        if retry > params.max_retries.unwrap_or(0) {
            return Err(CommonError::from(
                "Retry count should be less than max retries".to_owned(),
            ));
        }
        job.retries = Set(retry);
    }

    job.output = Set(params.output.clone());

    job.update(db)
        .await
        .map_err(|e| CommonError::from(e.to_string()))
}
