pub mod task_queue;
pub mod worker;

use chrono::{NaiveDateTime, Utc};
use entity::chunks::*;
use entity::files::*;
use entity::prelude::{Chunks, Files};
use entity::*;
use migration::sea_orm::prelude::Uuid;
use migration::sea_orm::ActiveValue::Set;
use migration::sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, InsertResult, ModelTrait,
};
use rocket::Data;
use rust_file_splitting_utils::file_merger::merge;
use sea_orm::IntoActiveModel;
use std::fs;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use teloxide::net::{download_file, Download};
use teloxide::payloads::GetFile;
use teloxide::prelude::{Request, Requester};
use teloxide::types::{ChatId, InputFile, Recipient};
use teloxide::RequestError::{Network, RetryAfter};
use teloxide::{Bot, DownloadError};
use tokio::time::{self, sleep, Duration};
pub async fn uploader(
    db: &DatabaseConnection,
    bot: &Bot,
    path: String,
    uuid: Uuid,
    file_id: i32,
) -> i32 {
    println!("Uploading file: {}", path);
    println!("Splitting...");
    let parts = rust_file_splitting_utils::file_splitter::split(
        path.clone(),
        19922944,
        Option::from(format!("{}{}", uuid, "/")),
    );

    //Uploading the pieces to telegram
    for (pos, e) in parts.iter().enumerate() {
        let pos = pos + 1;
        let mut success = false;
        while !success {
            let result = bot
                .send_document(
                    Recipient::Id(ChatId(1069912693)),
                    InputFile::file(PathBuf::from(e)),
                )
                .send()
                .await;
            match result {
                Ok(chunk_id) => {
                    println!("{:?}", chunk_id);
                    let single_part = entity::chunks::ActiveModel {
                        id: Default::default(),
                        telegram_file_id: Set(chunk_id.document().unwrap().file.id.clone()),
                        order: Set(pos as i32),
                        file: Set(file_id),
                    };
                    //Adding the chunk to the database
                    single_part.insert(db).await.unwrap();
                    //Delete the chunk from memory
                    fs::remove_file(e).unwrap();
                    success = true;
                }
                Err(error) => match error {
                    RetryAfter(retry_after) => {
                        sleep(Duration::from_secs(retry_after.as_secs() + 1)).await;
                    }
                    // Altri errori non gestiti vengono ritornati
                    _ => println!("{}", error.to_string()),
                },
            }
        }
    }

    fs::remove_dir(uuid.to_string()).unwrap();
    fs::remove_file(path).unwrap();
    file_id
}

pub async fn downloader(db: &DatabaseConnection, bot: &Bot, db_file_id: u64, uuid: Uuid) -> String {
    let file_info = Files::find_by_id(db_file_id as i32)
        .one(db)
        .await
        .unwrap()
        .unwrap();

    //now lazily fetch all the chunks
    let chunks = file_info.find_related(Chunks).all(db).await.unwrap();
    println!("Chunks :{:#?}", chunks);
    //Create a folder for all the chunks
    let chunk_dir = format!("./{}/", uuid);
    tokio::fs::create_dir(&chunk_dir).await.unwrap();
    //Initialize the vec containing all the chunks to be merged
    let mut chunk_path_list: Vec<String> = vec![];
    for i in chunks.iter() {
        let x = GetFile {
            file_id: i.telegram_file_id.clone(),
        };
        let file = bot.get_file(&i.telegram_file_id).await.unwrap();
        let file_path = format!("{}{}", chunk_dir, i.order);

        let mut dst = tokio::fs::File::create(&file_path).await.unwrap();

        let mut success = false;
        while !success {
            let download_result = bot.download_file(&file.path, &mut dst).await;
            match download_result {
                Ok(..) => {
                    chunk_path_list.push(file_path.clone());
                    println!("{:?}", x);
                    println!("{:?}", i);
                    success = true;
                }
                Err(error) => {
                    println!("{}", error.to_string());
                }
            }
        }
    }
    let result_dir = format!("{}{}", &chunk_dir, file_info.filename);
    println!("{}", result_dir);
    tokio::fs::File::create(format!("{}{}", "", file_info.filename)).await;
    merge(
        file_info.filename.clone(),
        "".to_string(),
        chunk_path_list.clone(),
        true,
    );
    for i in chunk_path_list.iter() {
        tokio::fs::remove_file(i).await;
    }
    tokio::fs::remove_dir(chunk_dir).await;
    //Update current file storage info and last download on db
    let mut active_file_info = file_info.clone().into_active_model();
    active_file_info.locally_stored = Set(Option::from(true));
    active_file_info.last_download = Set(Option::from(
        NaiveDateTime::from_timestamp_opt(Utc::now().timestamp(), 0)
            .unwrap()
            .to_string(),
    ));
    active_file_info.update(db).await.unwrap();
    file_info.filename
}
