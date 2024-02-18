use std::{env, io};
use std::fs::File;
use sea_orm::{Database, DatabaseConnection};

mod splitting_utils;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .init();

    //let parts = splitting_utils::file_splitter::split(String::from("nana.mkv"), 1024 * 1024 * 10, None);

    //splitting_utils::file_merger::merge(String::from("nana.mkv"), String::from("output/"),parts);
    let db: DatabaseConnection = Database::connect("sqlite://db.sqlite?mode=rwc").await.unwrap();


}
