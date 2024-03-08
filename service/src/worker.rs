use crate::task_queue::TaskType;
use crate::{downloader, uploader};
use chrono::{NaiveDateTime, Utc};
use entity::task_list;
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait};
use std::path::PathBuf;
use std::sync::Arc;
use teloxide::Bot;
use tokio::fs;
use tokio::sync::mpsc::{Receiver, UnboundedReceiver};

pub async fn worker(mut rx: UnboundedReceiver<TaskType>, db: Arc<DatabaseConnection>, bot: Bot) {
    while let Some(task) = rx.recv().await {
        match task {
            TaskType::Upload {
                id,
                file_path,
                user_id,
                file_name,
                file_id,
            } => {
                let task_from_db = task_list::Entity::find_by_id(id)
                    .one(db.as_ref())
                    .await
                    .unwrap();
                println!("{:?}", task_from_db);
                let mut new_status: task_list::ActiveModel = task_from_db.unwrap().into();
                new_status.status = Set(String::from("WORKING"));
                new_status = new_status.update(db.as_ref()).await.unwrap().into();
                println!("{} {} {} {} {}", id, file_path, user_id, file_name, file_id);
                let _ = uploader(db.as_ref(), &bot, file_path, id, file_id as i32).await;
                new_status.status = Set(String::from("COMPLETED"));
                new_status.completion_time = Set(Option::from(
                    NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0).to_string(),
                ));

                new_status.update(db.as_ref()).await.unwrap();
            }
            TaskType::Download {
                id,
                db_file_id,
                user_id,
            } => {
                let task_from_db = task_list::Entity::find_by_id(id)
                    .one(db.as_ref())
                    .await
                    .unwrap();
                println!("{:?}", task_from_db);
                let mut new_status: task_list::ActiveModel = task_from_db.unwrap().into();
                new_status.status = Set(String::from("WORKING"));
                new_status = new_status.update(db.as_ref()).await.unwrap().into();
                let _ = downloader(db.as_ref(), &bot, db_file_id, id).await;
                new_status.status = Set(String::from("COMPLETED"));
                new_status.completion_time = Set(Option::from(
                    NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0).to_string(),
                ));

                new_status.update(db.as_ref()).await.unwrap();
            }
        }
    }
}