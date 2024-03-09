use rocket::form::Form;
use rocket::serde::json::Json;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait};
use sea_orm_rocket::Connection;
use entity::files;
use crate::jwtauth::jwt::JWT;
use crate::pool::Db;
use crate::responses::NetworkResponse;
use sea_orm::QueryFilter;

#[derive(FromForm)]
struct NewDirForm{
    name:String,
    parent: i32
}
#[post("/createDir", data="<new_dir_input>")]
pub async fn new_dir_handler(
    conn: Connection<'_, Db>,
    mut new_dir_input: Form<NewDirForm>,
    key: Result<JWT, NetworkResponse>
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
            let original_parent = if new_dir_input.parent == -1 { None } else { Some(new_dir_input.parent) };

            let parent_dir = match original_parent {
                Some(dir_id) => files::Entity::find_by_id(dir_id)
                    .filter(files::Column::Type.eq(true)) //must be a folder
                    .filter(files::Column::User.eq(c.subject_id))
                    .one(&db.clone())
                    .await.unwrap(),
                None => None,
            };
            let valid_dir = match parent_dir {
                Some(d) => true, // La directory esiste ed è valida
                None if original_parent.is_none() => true, // dir era -1 o non specificata, usiamo la root directory
                None => false, // dir era specificata ma non valida
            };
            if valid_dir {
                //check if not exists dir with same name
                let exists = files::Entity::find()
                    .filter(files::Column::Filename.eq(new_dir_input.name.clone()))
                    .filter(files::Column::Type.eq(true)) // Assuming 'true' represents directories
                    .filter(
                        original_parent.map_or_else(
                            || files::Column::ParentDir.is_null(),
                            |parent_id| files::Column::ParentDir.eq(parent_id)
                        )
                    )
                    .one(&db.clone())
                    .await
                    .unwrap()
                    .is_some();
                if exists {
                    // If a directory with the same name exists, return an error
                    return Err(Json(NetworkResponse::BadRequest("Directory with the same name already exists".to_string())));
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
                        },
                        Err(e) => Err(Json(NetworkResponse::BadRequest(e.to_string())))
                    }
                }
            } else {
                Err(Json(NetworkResponse::NotFound("Parent dir not found".to_string())))
            }
        }
        Err(e) => Err(e)
    };
    response
}