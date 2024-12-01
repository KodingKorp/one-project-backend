use poem::{web::Data, Route};
use poem_openapi::{payload::Json, ApiResponse, Object, OpenApi, Tags};
use redis::{Client, ConnectionLike};
use serde::Deserialize;

mod e2e_test;

use crate::{
    bootstrap::AppState,
    capabilities::{
        background, lib::service_trait::Service, notifications::service::NotificationService,
    },
};

use self::background::orchestrator::BackgroundOrchestrator;

#[derive(Debug, Object, Clone, Eq, PartialEq, Deserialize)]
pub struct Ping {
    pub up: bool,
}
#[derive(Debug, Object, Clone, Eq, PartialEq, Deserialize)]
pub struct Health {
    pub db: bool,
    pub redis: bool,
    pub mailer: bool,
    pub background: bool,
}

#[derive(ApiResponse)]
pub enum PingResponse {
    #[oai(status = 200)]
    Ok(Json<Ping>),
}

#[derive(ApiResponse)]
pub enum HealthResponse {
    #[oai(status = 200)]
    Ok(Json<Health>),
}

#[derive(Tags)]
enum ApiTags {
    /// Operations about user
    PingResponse,
    HealthResponse,
}

#[derive(Default)]
pub struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/ping", method = "get", tag = "ApiTags::PingResponse")]
    pub async fn ping(&self) -> PingResponse {
        PingResponse::Ok(Json(Ping { up: true }))
    }

    #[oai(path = "/health", method = "get", tag = "ApiTags::HealthResponse")]
    pub async fn health(&self, state: Data<&AppState>, redis: Data<&Client>, orchestrator: Data<&BackgroundOrchestrator>) -> HealthResponse {
        // Redis
        let mut redis_state = false;
        match redis.get_connection() {
            Ok(mut connection) => redis_state = connection.check_connection(),
            Err(e) => {
                tracing::error!("{:#?}", e);
            }
        }

        // DB
        let mut db_state = false;
        match state.db.ping().await {
            Ok(_) => db_state = true,
            Err(e) => tracing::error!("{}", e),
        };

        // Mailer
        let mailer_state = NotificationService::check_mailer_connection().await;
        let background = orchestrator.health().await;
        HealthResponse::Ok(Json(Health {
            db: db_state,
            redis: redis_state,
            mailer: mailer_state,
            background,
        }))
    }
}

pub struct RootService;

#[async_trait::async_trait]
impl Service for RootService {
    fn register_routes() -> Option<impl OpenApi> {
        Some(Api)
    }
    async fn register_background(orchestrator: background::orchestrator::BackgroundOrchestrator) -> background::orchestrator::BackgroundOrchestrator {
        orchestrator
    }

    fn register_health_check(_: Route) -> Option<Route> {
        None
    }
}
