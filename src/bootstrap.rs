use std::time::Duration;

use crate::{
    base::RootService,
    capabilities::{
        background, database, iam::middleware::AuthorizationMiddleware, iam::service::IAMService,
        lib::service_trait::Service, logger, notifications::service::NotificationService,
    },
};
use poem::{
    endpoint::StaticFilesEndpoint,
    middleware::{
        AddDataEndpoint, CatchPanic, CatchPanicEndpoint, CookieJarManagerEndpoint, Cors,
        CorsEndpoint, Tracing, TracingEndpoint,
    },
    session::{CookieConfig, RedisStorage, ServerSession, ServerSessionEndpoint},
    web::cookie::SameSite,
    EndpointExt, Route,
};
use poem_openapi::OpenApiService;
use sea_orm::DatabaseConnection;

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
    logger::info("App State Created");
    let mut router = Route::new();
    logger::info("Router Created");
    let mut orchestrator =
        background::orchestrator::BackgroundOrchestrator::new(Some(state.clone()));
    // Set up services
    logger::info("Orchestrator Created");
    (router, orchestrator) = handle_services(router, orchestrator).await;
    logger::info("Services Registered");
    // Start background services
    orchestrator.start().await;
    logger::info("Background Services Started");

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
    logger::info("Router Built");
    a
}

async fn handle_services(
    mut router: Route,
    mut orchestrator: BackgroundOrchestrator,
) -> (Route, BackgroundOrchestrator) {
    // Handle Routes
    let api_list = (
        RootService::register_routes().unwrap(),
        IAMService::register_routes().unwrap(),
    );
    let private_api_list = 
        IAMService::register_private_routes().unwrap();
    let all_private_apis =
        OpenApiService::new(private_api_list, "Private APIs", "1.0").url_prefix("/api/v1");
    let private_ui = all_private_apis.swagger_ui();
    let private_swagger_yaml = all_private_apis.spec_endpoint_yaml();
    let all_apis = OpenApiService::new(api_list, "Prod APIs", "1.0").url_prefix("/api/v1/public");
    let all_private_apis = all_private_apis.with(AuthorizationMiddleware);

    let ui = all_apis.swagger_ui();
    let swagger_yaml = all_apis.spec_endpoint_yaml();
    router = router
        .nest("/", StaticFilesEndpoint::new("./static"))
        .nest("/api/v1/public", all_apis)
        .nest("/swagger", ui)
        .at("/swagger.yaml", swagger_yaml)
        .nest("/private/swagger", private_ui)
        .at("/private/swagger.yaml", private_swagger_yaml)
        .nest("/api/v1", all_private_apis);
    logger::info("Routes Registered");
    // Handle background services
    orchestrator = NotificationService::register_background(orchestrator).await;
    logger::info("Notification Service Registered");
    orchestrator = IAMService::register_background(orchestrator).await;
    logger::info("IAM Service Registered");
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
    let domain = std::env::var("COOKIE_DOMAIN").expect("COOKIE_DOMAIN is not set in .env file");
    let max_session = std::env::var("MAX_SESSION_DURATION")
        .expect("MAX_SESSION_DURATION is not set in .env file");
    let max_session: u64 = max_session
        .parse()
        .expect("MAX_SESSION_DURATION must be a valid number");
    let client = database::create_redis_client().await;
    let connection_manager = ConnectionManager::new(client.clone()).await.unwrap();

    logger::info("Connected to Redis");
    // log max session duration
    logger::info(&format!(
        "Max session duration set to: {} s, Duration {}",
        max_session,
        Duration::from_secs(max_session).as_secs_f64()
    ));

    (
        ServerSession::new(
            CookieConfig::default()
                .secure(true)
                .name("session")
                .http_only(true)
                .same_site(SameSite::None)
                .max_age(Duration::from_secs(max_session))
                .domain(domain),
            RedisStorage::new(connection_manager.clone()),
        ),
        client,
    )
}
