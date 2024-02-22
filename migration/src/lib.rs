pub use sea_orm_migration::prelude::*;

mod m20240221_184457_users;
mod m20240221_185739_files;
mod m20240221_232414_chunks;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240221_184457_users::Migration),
            Box::new(m20240221_185739_files::Migration),
            Box::new(m20240221_232414_chunks::Migration),
        ]
    }
}
