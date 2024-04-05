use std::path::Path;

use crate::jwtauth;
use crate::jwtauth::jwt::{Claims, JWT};
use crate::pool::Db;
use crate::responses::NetworkResponse;
use chrono::{NaiveDateTime, Utc};
use entity::{files, task_list};
use migration::Alias;
use rocket::form::Form;
use rocket::serde::json::Json;
use rocket_download_response::DownloadResponse;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, JoinType, QuerySelect,
};
use sea_orm::{IntoActiveModel, QueryFilter};
use sea_orm_rocket::Connection;
use serde_json::json;

#[get("/tasks")]
pub async fn get_all_tasks(
    conn: Connection<'_, Db>,
    key: Result<JWT, NetworkResponse>,
) -> Result<Json<NetworkResponse>, Json<NetworkResponse>> {
    let key = match key {
        Ok(JWT { claims: c }) => Ok(c),
        _ => Err(Json(NetworkResponse::Unauthorized(
            "Requested unauthorized".to_string(),
        ))),
    };

    let response = match key {
        Ok(c) => {
            let db: &&DatabaseConnection = &conn.into_inner();
            let tasks = task_list::Entity::find()
                .select_only()
                .column(task_list::Column::Id)
                .column(task_list::Column::File)
                .column(task_list::Column::Status)
                .column(task_list::Column::AddTime)
                .column(task_list::Column::CompletionTime)
                .column(task_list::Column::Type)
                .filter(files::Column::User.eq(c.subject_id))
                .find_also_related(files::Entity)
                .all(*db)
                .await
                .unwrap();

            //Build a json from vec
            let json_tasks: Vec<serde_json::Value> = tasks
                .into_iter()
                .map(|(task, file)| {
                    json!({
                        "id": task.id,
                        "name": task.file,
                        "status": task.status,
                        "add_time": task.add_time,
                        "completion_time": task.completion_time,
                        "type": task.r#type,
                        "filename": file.unwrap().filename.replace("./temp/", ""),
                    })
                })
                .collect();
            Ok(Json(NetworkResponse::Ok(
                serde_json::to_string(&json_tasks).unwrap(),
            )))
        }
        Err(e) => Err(e),
    };
    response
}
