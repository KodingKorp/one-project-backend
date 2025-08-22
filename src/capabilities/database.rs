use crate::logger;
use migration::{sea_orm, MigratorTrait};
use redis::aio::ConnectionManager;
use sea_orm::DatabaseConnection;
use std::time::Duration;

pub(crate) async fn make_pg_db_connection() -> DatabaseConnection {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let auto_migrate = std::env::var("AUTO_MIGRATE").expect("AUTO_MIGRATE is not set in .env file");

    let mut opts = sea_orm::ConnectOptions::new(db_url);
    opts.max_connections(10)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true);

    let conn = sea_orm::Database::connect(opts)
        .await
        .expect("Database connection failed");

    if auto_migrate == "TRUE" {
        match migration::Migrator::up(&conn, None).await {
            Ok(_) => logger::info("Migrations successful"),
            Err(e) => {
                logger::error("Migrations failed");
                panic!("{}", e); // panic if migrations fail on server start
            }
        }
    }
    conn
}

pub(crate) async fn create_redis_client() -> redis::Client {
    let redis_url = std::env::var("REDIS").expect("REDIS is not set in .env file");
    redis::Client::open(redis_url).unwrap()
}

pub(crate) async fn create_redis_connection_manager() -> redis::aio::ConnectionManager {
    let client = create_redis_client().await;
    let connection_manager = ConnectionManager::new(client.clone()).await.unwrap();
    connection_manager
}
