use crate::jwtauth::jwt::{Claims, JWT};
use crate::pool::Db;
use crate::responses::NetworkResponse;
use entity::files;
use entity::prelude::Files;
use rocket::form::Form;
use rocket::serde::json::Json;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryOrder,
};
use sea_orm::{IntoActiveModel, QueryFilter};
use sea_orm_rocket::Connection;
use serde_json::json;

pub async fn valid_file(db: &DatabaseConnection, file_id: &i32, user_id: &i32) -> bool {
    let exists = files::Entity::find()
        .filter(files::Column::Id.eq(*file_id))
        .filter(files::Column::User.eq(*user_id))
        .one(db)
        .await
        .unwrap()
        .is_some();
    exists
}

#[get("/deleteFile/<file_id>")]
pub async fn delete_file_handler(
    conn: Connection<'_, Db>,
    file_id: i32,
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
            let db = conn.into_inner();
            let uid = c.subject_id;

            let valid = valid_file(&db, &file_id, &uid).await;
            if valid {
                files::Entity::delete_by_id(*&file_id)
                    .filter(files::Column::Id.eq(file_id))
                    .exec(db)
                    .await
                    .unwrap();
                Ok(Json(NetworkResponse::Ok("File deleted".to_string())))
            } else {
                Err(Json(NetworkResponse::NotFound(
                    "File ID not found".to_string(),
                )))
            }
        }
        Err(e) => Err(e),
    };
    response
}
#[derive(FromForm)]
pub struct RenameFileForm {
    new_name: String,
}

#[post("/renameFile/<file_id>", data = "<new_name>")]
pub async fn rename_file_handler(
    conn: Connection<'_, Db>,
    file_id: i32,
    new_name: Form<RenameFileForm>,
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
            let db = conn.into_inner();
            let uid = c.subject_id;

            let valid = valid_file(&db, &file_id, &uid).await;
            if valid {
                let file = files::Entity::find_by_id(file_id)
                    .one(&db.clone())
                    .await
                    .unwrap();

                //Convert it into an active model
                let mut active_file = file.unwrap().into_active_model();
                active_file.filename = Set(format!("./temp/{}", new_name.new_name));
                //Update the filename
                active_file.update(db).await.unwrap();

                Ok(Json(NetworkResponse::Ok("File renamed".to_string())))
            } else {
                Err(Json(NetworkResponse::NotFound(
                    "File ID not found".to_string(),
                )))
            }
        }
        Err(e) => Err(e),
    };
    response
}
