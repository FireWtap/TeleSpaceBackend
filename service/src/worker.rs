use crate::task_queue::TaskType;
use crate::{downloader, uploader};
use chrono::{NaiveDateTime, Utc};
use entity::{notification_tokens, task_list};
use fcm_v1::{message, Client};
use sea_orm::entity::prelude::ColumnTrait;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use teloxide::Bot;
use tokio::sync::mpsc::UnboundedReceiver;

pub async fn worker(
    mut rx: UnboundedReceiver<TaskType>,
    db: Arc<DatabaseConnection>,
    message_client: Client,
) {
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

                //Notification handler
                let mut tokens = notification_tokens::Entity::find()
                    .filter(notification_tokens::Column::User.eq(user_id as i32))
                    .all(db.as_ref())
                    .await
                    .unwrap();
                let mut notification = message::Notification::default();
                notification.title = Some("An upload has been completed!".to_string());
                notification.body = Some("Your file is now safely stored on Telegram. You can download it back whenever you want!:)".to_string());
                for token in tokens.iter_mut() {
                    let mut message = message::Message::default();
                    message.notification = Some(notification.clone());
                    message.token = Some(token.token_notification.clone());
                    //Send the notification
                    let response = message_client.send(&message).await.unwrap();
                    println!("Sending message to token: {:?}", token.token_notification);
                }
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

                let mut tokens = notification_tokens::Entity::find()
                    .filter(notification_tokens::Column::User.eq(user_id as i32))
                    .all(db.as_ref())
                    .await
                    .unwrap();
                let mut notification = message::Notification::default();
                notification.title = Some("A download has been completed!".to_string());
                notification.body = Some("Your file has been downloaded to our server, check out the dashboard for more info!".to_string());
                for token in tokens.iter_mut() {
                    let mut message = message::Message::default();
                    message.notification = Some(notification.clone());
                    message.token = Some(token.token_notification.clone());
                    //Send the notification
                    let response = message_client.send(&message).await.unwrap();
                    println!("Sending message to token: {:?}", token.token_notification);
                }
                //let mut builder = fcm::MessageBuilder::new("qO1YFT2VzpkelpQHVbKzMOezJTgjM3ZA3hpONQWGLFc", user_token);
            }
        }
    }
}
