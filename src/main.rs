use sea_orm::{ActiveModelTrait, ActiveValue, Database, DatabaseConnection, DbConn, DbErr, EntityTrait};
use std::fs::File;
use std::{env, io};
use sea_orm::ActiveValue::Set;
use migration::{Migrator, MigratorTrait};
use entity::*;
use entity::prelude::Users;


fn main() {
    let path = env::current_dir().unwrap();
    println!("The current directory is {}", path.display());
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .init();
    api::main();
    //let parts = rust_file_splitting_utils::file_splitter::split(String::from("nana.mkv"), 1024 * 1024 * 10, None);

    //rust_file_splitting_utils::file_merger::merge(String::from("nana.mkv"), String::from("output/"),parts);

    // db = establish_connection().await.unwrap();
    let user = users::ActiveModel{
        id: Default::default(),
        email: Set(String::from("Massafra32@gmail.com")),
        password_hash: Set(String::from("Pasqi")),
    };
    //let user = user.insert(&db).await.unwrap();

    //let users = Users::find().all(&db).await.unwrap();
    //println!("{:?}", users)
}


pub async fn establish_connection() -> Result<DbConn, DbErr> {
    let db = Database::connect("sqlite://db.sqlite?mode=rwc")
        .await
        .expect("Failed to setup the database");
    Migrator::up(&db, None)
        .await
        .expect("Failed to run migrations for tests");

    Ok(db)
}