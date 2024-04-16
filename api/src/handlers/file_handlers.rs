use std::path::Path;

use crate::jwtauth;
use crate::jwtauth::jwt::{Claims, JWT};
use crate::pool::Db;
use crate::responses::NetworkResponse;
use chrono::{NaiveDateTime, Utc};
use entity::files;
use rocket::form::Form;
use rocket::serde::json::Json;
use rocket_download_response::DownloadResponse;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait};
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

#[delete("/deleteFile/<file_id>")]
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
                clear_cache(db, file_id, uid).await;

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
                let mut active_file: files::ActiveModel = file.unwrap().into_active_model();
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
#[get("/downloadLocalFile/<token>/<file_id>")]
pub async fn locally_stored_download_handler(
    conn: Connection<'_, Db>,
    file_id: i32,
    token: String,
) -> Result<DownloadResponse, Json<NetworkResponse>> {
    // First, handle the key validation and extract claims if valid.
    let claims = jwtauth::jwt::decode_jwt(token);
    let key = match claims {
        Ok(c) => Ok(c),
        Err(e) => Err(Json(NetworkResponse::Unauthorized(
            "Not authorized".to_string(),
        ))),
    };
    let response = match key {
        Ok(c) => {
            let db = conn.into_inner();
            let uid = c.subject_id;

            // Check if the file is valid for the given user.
            let valid = valid_file(&db, &file_id, &uid).await;
            if !valid {
                return Err(Json(NetworkResponse::NotFound(
                    "File ID not found".to_string(),
                )));
            }

            // Attempt to retrieve the file from the database.
            match files::Entity::find_by_id(file_id).one(db).await {
                Ok(Some(file)) if file.locally_stored.unwrap_or(false) => {
                    let filename = file.filename.clone();
                    let file_path = Path::new(&filename);

                    match DownloadResponse::from_file(
                        file_path,
                        Some(filename.replace("./temp/", "")),
                        None,
                    )
                    .await
                    {
                        Ok(response) => {
                            //Let's update the file last download date so that we can run a cron job to delete files that have not been downloaded for a long time
                            let mut active_file = file.into_active_model();
                            active_file.last_download = Set(Option::from(
                                NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0)
                                    .to_string(),
                            ));
                            active_file.update(db).await.unwrap();

                            Ok(response)
                        }
                        Err(err) => Err(Json(NetworkResponse::BadRequest(format!(
                            "Error downloading file: {}",
                            err
                        )))),
                    }
                }
                Ok(Some(_)) => Err(Json(NetworkResponse::BadRequest(
                    "File not available for download".to_string(),
                ))),
                Ok(None) => Err(Json(NetworkResponse::NotFound(
                    "File not found".to_string(),
                ))),
                Err(err) => Err(Json(NetworkResponse::BadRequest(err.to_string()))),
            }
        }
        Err(e) => Err(e),
    };
    response
}

#[get("/info/<file_id>")]
pub async fn file_info_handler(
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
                match files::Entity::find_by_id(file_id).one(db).await {
                    Ok(Some(file)) => {
                        let file_info = json!({
                            "filename": file.filename,
                            "size": file.original_size,
                            "locally_stored": file.locally_stored,
                            "last_download":file.last_download,
                            "type": file.r#type,
                        });
                        Ok(Json(NetworkResponse::Ok(file_info.to_string())))
                    }
                    Ok(None) => Err(Json(NetworkResponse::NotFound(
                        "File not found".to_string(),
                    ))),
                    Err(err) => Err(Json(NetworkResponse::BadRequest(err.to_string()))),
                }
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

// if file is locally avaialble, provide a clear cache to delete the file from the server, first with a simple method, then with a proper route
pub async fn clear_cache(db: &DatabaseConnection, file_id: i32, user_id: i32) -> bool {
    let file = files::Entity::find_by_id(file_id)
        .one(db)
        .await
        .unwrap()
        .unwrap();

    let filename = file.filename.clone();
    let locally_available = file.locally_stored.unwrap_or(false);

    if (!locally_available) {
        return false;
    }
    let file_path = Path::new(&filename);

    tokio::fs::remove_file(file_path).await.unwrap();
    //Update the file to reflect that it is no longer locally stored
    let mut active_file = file.into_active_model();
    active_file.locally_stored = Set(Option::from(false));
    active_file.update(db).await.unwrap();
    return true;
}

#[get("/clearCache/<file_id>")]
pub async fn clear_cache_handler(
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
                let cleared = clear_cache(&db, file_id, uid).await;
                if cleared {
                    Ok(Json(NetworkResponse::Ok("Cache cleared".to_string())))
                } else {
                    Err(Json(NetworkResponse::BadRequest(
                        "File not locally stored".to_string(),
                    )))
                }
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
