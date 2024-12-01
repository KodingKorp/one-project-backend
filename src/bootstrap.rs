use crate::{
    base::RootService,
    capabilities::{
        background,
        database,
    },
};
use poem::{
    middleware::{
        AddDataEndpoint, CatchPanic, CatchPanicEndpoint, CookieJarManagerEndpoint, Cors,
        CorsEndpoint, Tracing, TracingEndpoint,
    },
    session::{CookieConfig, RedisStorage, ServerSession, ServerSessionEndpoint},
    EndpointExt, Route,
};
use poem_openapi::OpenApiService;
use sea_orm::DatabaseConnection;

use crate::capabilities::{self, lib::service_trait::Service, logger};

use redis::{aio::ConnectionManager, Client};

use self::background::BackgroundOrchestrator;

pub type App = CatchPanicEndpoint<
    TracingEndpoint<
        CorsEndpoint<
            CookieJarManagerEndpoint<
                ServerSessionEndpoint<
                    RedisStorage<ConnectionManager>,
                    AddDataEndpoint<
                        AddDataEndpoint<AddDataEndpoint<Route, AppState>, Client>,
                        BackgroundOrchestrator,
                    >,
                >,
            >,
        >,
    >,
    (),
>;

pub async fn build_app() -> App {
    let (state, (server_session, redis)) = tokio::join!(make_app_state(), make_server_session());

    let mut router = Route::new();
    let mut orchestrator =
        background::orchestrator::BackgroundOrchestrator::new(Some(state.clone()));
    // Set up services
    (router, orchestrator) = handle_services(router, orchestrator).await;

    // Start background services
    orchestrator.start().await;

    let a = router
        .data(state)
        .data(redis)
        .data(orchestrator)
        .with(server_session)
        .with(
            Cors::new()
                .allow_methods(["OPTIONS", "GET", "POST", "PUT", "DELETE"])
                .allow_credentials(true),
        )
        .with(Tracing)
        .with(CatchPanic::new());
    a
}

async fn handle_services(
    mut router: Route,
    mut orchestrator: BackgroundOrchestrator,
) -> (Route, BackgroundOrchestrator) {
    // Handle Routes
    let api_list = (
        RootService::register_routes().unwrap(),
        capabilities::iam::service::IAMService::register_routes().unwrap(),
    );
    let all_apis = OpenApiService::new(api_list, "Prod APIs", "1.0").url_prefix("/api/v1");
    let ui = all_apis.swagger_ui();
    let swagger_yaml = all_apis.spec_endpoint_yaml();
    router = router
        .nest("/api/v1", all_apis)
        .nest("/swagger", ui)
        .at("/swagger.yaml", swagger_yaml);

    // Handle background services
    orchestrator = capabilities::notifications::service::NotificationService::register_background(
        orchestrator,
    )
    .await;
    orchestrator = capabilities::iam::service::IAMService::register_background(orchestrator).await;
    (router, orchestrator)
}

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}
async fn make_app_state() -> AppState {
    let db = database::make_pg_db_connection().await;
    logger::info("DB Connection Created");
    AppState { db: db.clone() }
}

async fn make_server_session() -> (ServerSession<RedisStorage<ConnectionManager>>, Client) {
    let env = std::env::var("ENV").expect("ENV is not set in .env file");

    let client = database::create_redis_client().await;
    let connection_manager = ConnectionManager::new(client.clone()).await.unwrap();

    logger::info("Connected to Redis");

    (
        ServerSession::new(
            CookieConfig::default().secure(env == "production"),
            RedisStorage::new(connection_manager.clone()),
        ),
        client,
    )
}
