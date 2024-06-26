


use crate::jwtauth::jwt::{JWT};
use crate::pool::Db;
use crate::responses::NetworkResponse;

use entity::{files, users};

use rocket::form::Form;
use rocket::serde::json::Json;

use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, InsertResult,
    PaginatorTrait,
};
use sea_orm::{IntoActiveModel, QueryFilter};
use sea_orm_rocket::Connection;
use serde_json::json;
use teloxide::requests::Requester;
use teloxide::types::ChatId;
use teloxide::{Bot};

use crate::utils::encrypt_password;

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
                "chatId": user.user_telegram_id,
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
        Ok(_c) => {
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

#[derive(FromForm)]
pub struct NewBotToken {
    bot_token: String,
}
#[post("/updateBotToken", data = "<form>")]
pub async fn update_token_bot_handler(
    conn: Connection<'_, Db>,
    key: Result<JWT, NetworkResponse>,
    form: Form<NewBotToken>,
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
            let user: users::Model = users::Entity::find_by_id(c.subject_id)
                .one(db)
                .await
                .unwrap()
                .unwrap(); //We're sure it exists
            let mut user_active = user.into_active_model();
            user_active.bot_token = Set(form.bot_token.clone()); //Change token
            let updated_user = user_active.update(db).await.unwrap(); //Persist update
                                                                      // let's send a new message to the user
            let bot = Bot::new(&form.bot_token);
            let _ = bot.send_message(ChatId(updated_user.user_telegram_id.clone()), "Hey! you successfully connected your bot to TeleSpace🚀.\nKeep in mind we can't assure all your data are safe now, for the moment at least").await;
            Ok(Json(NetworkResponse::Ok("Bot token updated".to_string())))
        }
        Err(e) => Err(e),
    };
    response
}

#[derive(FromForm)]

pub struct NewChatId {
    chat_id: i64,
}
#[post("/updateChatId", data = "<form>")]
pub async fn update_chat_id_handler(
    conn: Connection<'_, Db>,
    key: Result<JWT, NetworkResponse>,
    form: Form<NewChatId>,
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
            let user: users::Model = users::Entity::find_by_id(c.subject_id)
                .one(db)
                .await
                .unwrap()
                .unwrap(); //We're sure it exists
            let user_clone = user.clone();

            let mut user_active = user.into_active_model();
            user_active.user_telegram_id = Set(form.chat_id); //Change token
            let _ = user_active.update(db).await.unwrap(); //Persist update

            //Inform the user by sending a message
            let bot = Bot::new(user_clone.bot_token);
            let _ = bot
                .send_message(
                    ChatId(form.chat_id),
                    "Hey! you successfully connected your chat to TeleSpace🚀.\nKeep in mind we can't assure all your data are safe now, for the moment at least",
                )
                .await;
            Ok(Json(NetworkResponse::Ok("Chat id updated".to_string())))
        }
        Err(e) => Err(e),
    };
    response
}

#[get("/stats")]
pub async fn get_personal_stats_handler(
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
            // Numbers of uploaded file
            // Cumulative dimension of uploaded files
            let db = conn.into_inner();
            let file_count = files::Entity::find()
                .filter(files::Column::User.eq(c.subject_id))
                .filter(files::Column::Type.eq(0))
                .count(db)
                .await
                .unwrap();
            let file_sizes = files::Entity::find()
                .filter(files::Column::User.eq(c.subject_id))
                .filter(files::Column::Type.eq(0))
                .all(db)
                .await
                .unwrap();
            let cumulative_size = file_sizes.iter().map(|el| el.original_size).sum::<i64>();

            let res_json = json!({
                "file_count": file_count,
                "cumulative_size": cumulative_size,
            });
            Ok(Json(NetworkResponse::Ok(res_json.to_string())))
        }
        Err(e) => Err(e),
    };
    response
}

//Register! T_T

#[derive(FromForm)]
pub struct RegisterForm {
    email: String,
    password: String,
    bot_token: String,
    chat_id: i64,
}
#[post("/register", data = "<form>")]
pub async fn register_user_handler(
    conn: Connection<'_, Db>,
    form: Form<RegisterForm>,
) -> Result<Json<NetworkResponse>, Json<NetworkResponse>> {
    let db = conn.into_inner();

    if !form.email.contains('@') {
        return Ok(Json(NetworkResponse::BadRequest(
            "Invalid email address".to_string(),
        )));
    }

    if form.password.len() < 8 {
        return Ok(Json(NetworkResponse::BadRequest(
            "Password must be at least 8 characters long".to_string(),
        )));
    }

    //Check for existing user
    let existing_user = users::Entity::find()
        .filter(users::Column::Email.eq(&form.email))
        .one(db)
        .await
        .unwrap();
    if existing_user.is_some() {
        return Ok(Json(NetworkResponse::Conflict(
            "User already exists".to_string(),
        )));
    }

    let user = users::ActiveModel {
        email: Set(form.email.clone()),
        password_hash: Set(encrypt_password(form.password.clone())),
        bot_token: Set(form.bot_token.clone()),
        user_telegram_id: Set(form.chat_id),
        ..Default::default()
    };
    let res: InsertResult<users::ActiveModel> = users::Entity::insert(user).exec(db).await.unwrap();

    Ok(Json(NetworkResponse::Ok(
        "User Id: ".to_string() + &res.last_insert_id.to_string(),
    )))
}
