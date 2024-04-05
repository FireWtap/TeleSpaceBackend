use crate::jwtauth::jwt::{Claims, JWT};
use crate::pool::Db;
use crate::responses::NetworkResponse;
use entity::files;
use entity::prelude::Files;
use rocket::form::Form;
use rocket::serde::json::Json;
use sea_orm::ActiveValue::Set;
use sea_orm::QueryFilter;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryOrder};
use sea_orm_rocket::Connection;
use serde_json::json;

pub async fn valid_parent(db: &DatabaseConnection, parent_id: &Option<i32>, uid: &i32) -> bool {
    let parent_dir = match parent_id {
        Some(dir_id) => files::Entity::find_by_id(*dir_id)
            .filter(files::Column::Type.eq(true)) //must be a folder
            .filter(files::Column::User.eq(*uid))
            .one(db)
            .await
            .unwrap(),
        None => None,
    };
    let valid_dir = match parent_dir {
        Some(d) => true,                      // La directory esiste ed è valida
        None if parent_dir.is_none() => true, // dir era -1 o non specificata, usiamo la root directory
        None => false,                        // dir era specificata ma non valida
    };
    valid_dir
}

pub async fn check_dir_exists(db: &DatabaseConnection, dir_id: &i32) -> bool {
    let exists = files::Entity::find_by_id(*dir_id)
        .filter(files::Column::Type.eq(true)) // Assuming 'true' represents directories
        .one(db)
        .await
        .unwrap()
        .is_some();
    exists
}

#[derive(FromForm)]
pub struct NewDirForm {
    name: String,
    parent: i32,
}
#[post("/createDir", data = "<new_dir_input>")]
pub async fn new_dir_handler(
    conn: Connection<'_, Db>,
    mut new_dir_input: Form<NewDirForm>,
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
            let original_parent = if new_dir_input.parent == -1 {
                None
            } else {
                Some(new_dir_input.parent)
            };

            let valid_dir = valid_parent(&db.clone(), &original_parent, &c.subject_id).await;

            if valid_dir {
                //check if not exists dir with same name
                let exists = files::Entity::find()
                    .filter(files::Column::Filename.eq(new_dir_input.name.clone()))
                    .filter(files::Column::Type.eq(true)) // Assuming 'true' represents directories
                    .filter(original_parent.map_or_else(
                        || files::Column::ParentDir.is_null(),
                        |parent_id| files::Column::ParentDir.eq(parent_id),
                    ))
                    .one(&db.clone())
                    .await
                    .unwrap()
                    .is_some();
                if exists {
                    // If a directory with the same name exists, return an error
                    return Err(Json(NetworkResponse::BadRequest(
                        "Directory with the same name already exists".to_string(),
                    )));
                } else {
                    // If no such directory exists, proceed with creating the new directory
                    let file = files::ActiveModel {
                        id: Default::default(),
                        filename: Set(new_dir_input.name.clone()),
                        r#type: Set(true), // Assuming 'true' represents directories
                        user: Set(c.subject_id),
                        parent_dir: Set(original_parent.map(|dir| dir as i32)),
                        original_size: Set(0),
                        ..Default::default()
                    };

                    match files::Entity::insert(file).exec(db).await {
                        Ok(res) => {
                            let file_id = res.last_insert_id;
                            Ok(Json(NetworkResponse::Ok(file_id.to_string())))
                        }
                        Err(e) => Err(Json(NetworkResponse::BadRequest(e.to_string()))),
                    }
                }
            } else {
                Err(Json(NetworkResponse::NotFound(
                    "Parent dir not found".to_string(),
                )))
            }
        }
        Err(e) => Err(e),
    };
    response
}

#[get("/listDirectory/<id>")]
pub async fn list_directory(
    conn: Connection<'_, Db>,
    id: i32,
    key: Result<JWT, NetworkResponse>,
) -> Result<Json<NetworkResponse>, Json<NetworkResponse>> {
    let key = match key {
        Ok(JWT { claims: c }) => Ok(c),
        _ => Err(Json(NetworkResponse::Unauthorized(
            "Requested unauthorized".to_string(),
        ))),
    };
    let db = conn.into_inner();
    let response = match key {
        Ok(c) => {
            let original_parent = if id == -1 { None } else { Some(id) };
            if (original_parent.is_none()) {
                //list root dir
                let list_root = files::Entity::find()
                    .filter(files::Column::User.eq(c.subject_id))
                    .filter(files::Column::ParentDir.is_null())
                    .order_by_desc(files::Column::Type)
                    .all(db)
                    .await
                    .unwrap();
                Ok(Json(NetworkResponse::Ok(json!(list_root).to_string())))
            } else if (check_dir_exists(&db, &id).await) {
                //list all in that directory
                /*let dir_list = files::Entity::find_by_id(id)
                .filter(files::Column::Type.eq(true))
                .one(&db)
                .await.
                unwrap().unwrap();*/

                let childrens = files::Entity::find()
                    .filter(files::Column::ParentDir.eq(id))
                    .all(db)
                    .await
                    .unwrap();
                //dir_list.find_related(Files).all(db).await.unwrap();
                Ok(Json(NetworkResponse::Ok(json!(childrens).to_string())))
            } else {
                Err(Json(NetworkResponse::NotFound(String::from(
                    "Directory not found",
                ))))
            }
        }
        Err(e) => Err(e),
    };
    response
}

#[get("/getDirectoryName/<id>")]
pub async fn get_directory_name_handler(
    conn: Connection<'_, Db>,
    id: i32,
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
            if (check_dir_exists(&db, &id)).await {
                let name = Files::find_by_id(id)
                    .one(db)
                    .await
                    .unwrap()
                    .unwrap()
                    .filename;
                Ok(Json(NetworkResponse::Ok(name)))
            } else {
                Err(Json(NetworkResponse::NotFound(String::from(
                    "Directory doesn't exists",
                ))))
            }
        }
        Err(e) => Err(e),
    };
    response
}

#[get("/getParentDirectory/<id>")]
pub async fn get_parent_directory(
    conn: Connection<'_, Db>,
    id: i32,
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
            if check_dir_exists(&db, &id).await {
                let parent = files::Entity::find_by_id(id)
                    .filter(files::Column::User.eq(c.subject_id))
                    .one(db)
                    .await
                    .unwrap()
                    .unwrap();
                match parent.parent_dir {
                    Some(p) => Ok(Json(NetworkResponse::Ok(p.to_string()))),
                    None => Ok(Json(NetworkResponse::Ok("-1".to_string()))),
                }
            } else {
                Ok(Json(NetworkResponse::NotFound(
                    "Directory not found".to_string(),
                )))
            }
        }
        Err(e) => Err(e),
    };
    response
}

#[delete("/deleteDirectory/<id>")]
pub async fn delete_directory_handler(
    conn: Connection<'_, Db>,
    id: i32,
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
            if (check_dir_exists(&db, &id)).await {
                Files::delete_by_id(id)
                    .filter(files::Column::User.eq(c.subject_id))
                    .exec(db)
                    .await
                    .unwrap();
                Ok(Json(NetworkResponse::Ok("Directory deleted".to_string())))
            } else {
                Err(Json(NetworkResponse::NotFound(String::from(
                    "Directory doesn't exists",
                ))))
            }
        }
        Err(e) => Err(e),
    };
    response
}
