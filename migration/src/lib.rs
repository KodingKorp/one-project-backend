pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_user_table;
mod m20240809_114718_sessions;
mod m20241101_172249_user_login;
mod m20241103_032650_create_jobs;
mod m20241129_115216_add_queue_jobs;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_user_table::Migration),
            Box::new(m20240809_114718_sessions::Migration),
            Box::new(m20241101_172249_user_login::Migration),
            Box::new(m20241103_032650_create_jobs::Migration),
            Box::new(m20241129_115216_add_queue_jobs::Migration),
        ]
    }
}
