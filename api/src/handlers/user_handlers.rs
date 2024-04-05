use std::path::Path;

use crate::jwtauth;
use crate::jwtauth::jwt::{Claims, JWT};
use crate::pool::Db;
use crate::responses::NetworkResponse;
use chrono::{NaiveDateTime, Utc};
use entity::{files, task_list, users};
use migration::{Alias, RcOrArc};
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
use teloxide::requests::Requester;
use teloxide::Bot;

#[get("/me")]
pub async fn get_me_handler(
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
            let db = conn.into_inner();
            let user = users::Entity::find()
                .filter(users::Column::Id.eq(c.subject_id))
                .one(db)
                .await
                .unwrap()
                .unwrap();
            let res_json = json!({
                "id": user.id,
                "username": user.email,
                "botToken": user.bot_token,

            });
            Ok(Json(NetworkResponse::Ok(res_json.to_string())))
        }
        Err(e) => Err(e),
    };
    response
}

#[derive(FromForm)]
pub struct CheckBotToken {
    bot_token: String,
}
#[post("/checkBotToken", data = "<form>")]
pub async fn check_bottoken_validity(
    key: Result<JWT, NetworkResponse>,
    form: Form<CheckBotToken>,
) -> Result<Json<NetworkResponse>, Json<NetworkResponse>> {
    let key = match key {
        Ok(JWT { claims: c }) => Ok(c),
        _ => Err(Json(NetworkResponse::Unauthorized(
            "Requested unauthorized".to_string(),
        ))),
    };
    let response = match key {
        Ok(c) => {
            let bot = Bot::new(&form.bot_token);
            let me = bot.get_me().await;
            match me {
                Ok(_) => Ok(Json(NetworkResponse::Ok("true".to_string()))),
                Err(_) => Ok(Json(NetworkResponse::Ok("false".to_string()))),
            }
        }
        Err(e) => Err(e),
    };
    response
}
