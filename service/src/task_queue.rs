use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sea_orm::{Database, DatabaseConnection, EntityTrait, InsertResult};
use sea_orm::ActiveValue::Set;
use tokio::sync::mpsc;
use entity::{files, task_list};
use entity::prelude::TaskList;


#[derive(Debug, Serialize, Deserialize)]
pub enum TaskType {
    Download {
        id: Uuid,
        db_file_id: u64,
        user_id: u64,
    },
    Upload {
        id: Uuid,
        file_path: String,
        user_id: u64,
        file_name: String,
        file_id: u64,
    },
}

pub struct TaskQueue {
    sender: mpsc::UnboundedSender<TaskType>,
    db_conn: Arc<DatabaseConnection>, // Assuming DatabaseConnection is the type for your DB connection

}

impl TaskQueue {
    pub async fn new() -> (Self, mpsc::UnboundedReceiver<TaskType>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let db_conn = Database::connect("sqlite://db.sqlite?mode=rwc").await.unwrap();
        (TaskQueue { sender, db_conn:Arc::new(db_conn) }, receiver)
    }

    pub async fn add_task(&self, task: TaskType) -> Result<(), &'static str> {
        //Insert task into DB
        match task{
            TaskType::Upload { id,file_id, .. } => {let task = entity::task_list::ActiveModel{
                id: Set(id.to_string()),
                file: Set(file_id as i32),
                status: Set("WAITING".to_string()),
            };
            TaskList::insert(task).exec(self.db_conn.as_ref()).await.unwrap();
            }
            TaskType::Download { .. } =>{}
        }


        self.sender.send(task).map_err(|_| "Failed to add task")
    }
}
