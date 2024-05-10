use crate::task_queue::TaskType;
use crate::{downloader, uploader};
use chrono::{NaiveDateTime, Utc};
use entity::{notification_tokens, task_list};
use sea_orm::entity::prelude::ColumnTrait;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use teloxide::Bot;
use tokio::sync::mpsc::UnboundedReceiver;

pub async fn worker(mut rx: UnboundedReceiver<TaskType>, db: Arc<DatabaseConnection>) {
    while let Some(task) = rx.recv().await {
        match task {
            TaskType::Upload {
                id,
                file_path,
                user_id,
                file_name,
                file_id,
            } => {
                // We received a task but we're gonna use USER PERSONAL BOT TOKEN TO EXECUTE THE TASK.
                // fetching the bot token from the database
                let user = entity::users::Entity::find_by_id(user_id.clone() as i32)
                    .one(db.as_ref())
                    .await
                    .unwrap()
                    .unwrap();
                let bot = Bot::new(user.bot_token);
                let task_from_db = task_list::Entity::find_by_id(id)
                    .one(db.as_ref())
                    .await
                    .unwrap();
                println!("{:?}", task_from_db);
                let mut new_status: task_list::ActiveModel = task_from_db.unwrap().into();
                new_status.status = Set(String::from("WORKING"));
                new_status = new_status.update(db.as_ref()).await.unwrap().into();
                println!("{} {} {} {} {}", id, file_path, user_id, file_name, file_id);
                let _ = uploader(
                    db.as_ref(),
                    &bot,
                    user.user_telegram_id,
                    file_path,
                    id,
                    file_id as i32,
                )
                .await;
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
                let user = entity::users::Entity::find_by_id(user_id.clone() as i32)
                    .one(db.as_ref())
                    .await
                    .unwrap()
                    .unwrap();
                let bot = Bot::new(user.bot_token);
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
                //Hey the file has been downloaded! We can send a notification to the user now!
                let client = fcm::Client::new();
                let mut tokens = notification_tokens::Entity::find()
                    .filter(notification_tokens::Column::User.eq(user_id as i32))
                    .all(db.as_ref())
                    .await
                    .unwrap();

                for token in tokens.iter_mut() {
                    let mut message = fcm::NotificationBuilder::new();
                    message.title("Download Complete! Check your cloud");
                    message.body("Your file has been downloaded!");
                    let notification = message.finalize();
                    let api_key =
                        env::var("FIREBASE_SERVER_KEY").expect("FIREBASE_SERVER_KEY not found");

                    let mut message_builder =
                        fcm::MessageBuilder::new(api_key.as_str(), &token.token_notification);
                    println!("{:?}", token.token_notification);
                    message_builder.notification(notification);
                    let response = client.send(message_builder.finalize()).await;
                }
                //let mut builder = fcm::MessageBuilder::new("qO1YFT2VzpkelpQHVbKzMOezJTgjM3ZA3hpONQWGLFc", user_token);
            }
        }
    }
}
