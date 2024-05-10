pub use sea_orm_migration::prelude::*;

mod m20240221_184457_users;
mod m20240221_185740_files;
mod m20240221_232414_chunks;
mod m20240303_231056_task_list;
mod m20240510_142918_notification_token;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240221_184457_users::Migration),
            Box::new(m20240221_185740_files::Migration),
            Box::new(m20240221_232414_chunks::Migration),
            Box::new(m20240303_231056_task_list::Migration),
            Box::new(m20240510_142918_notification_token::Migration),
        ]
    }
}
