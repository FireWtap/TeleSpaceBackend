#[macro_use]
extern crate rocket;

use dotenvy::dotenv;
use rocket::data::{ByteUnit, Limits};
use rocket::fairing::{self, AdHoc};
use rocket::form::Form;
use rocket::fs::{relative, FileName, FileServer, TempFile};
use rocket::response::status::NotFound;
use rocket::response::{content, status};
use rocket::serde::json::Json;
use rocket::serde::Deserialize;
use rocket::{Build, Config, Response, Rocket, State};
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Cursor, DatabaseConnection, DbErr, EntityTrait, InsertResult,
    QueryFilter,
};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use migration::MigratorTrait;
use sea_orm_rocket::{Connection, Database};
use teloxide::payloads::SendDocument;
use teloxide::prelude::{Message, Requester};
use teloxide::types::{ChatId, InputFile, Recipient};
use teloxide::Bot;
use tokio::sync::Mutex;

mod jwtauth;
mod pool;
mod responses;
mod utils;

use pool::Db;

use crate::jwtauth::jwt::{create_jwt, JWT};
use crate::responses::ResponseBody::AuthToken;
use crate::responses::{NetworkResponse, ResponseBody};
use entity::files::ActiveModel;
use entity::prelude::Users;
use entity::users::Model;
pub use entity::*;
use service::downloader;

const DEFAULT_POSTS_PER_PAGE: u64 = 5;

#[get("/")]
async fn root(conn: Connection<'_, Db>) -> content::RawJson<String> {
    let db = conn.into_inner();

    let user = users::ActiveModel {
        id: Default::default(),
        email: Set(String::from("Massafra32@gmail.com")),
        password_hash: Set(String::from("Pasqi")),
    };
    let users = Users::find().into_json().all(db).await.unwrap();
    content::RawJson(format!("users:'{:?}'", users))
}
/*
#[get("/new")]
async fn new() -> Template {
    Template::render("new", &Context::default())
}

#[post("/", data = "<post_form>")]
async fn create(conn: Connection<'_, Db>, post_form: Form<post::Model>) -> Flash<Redirect> {
    let db = conn.into_inner();

    let form = post_form.into_inner();

    Flash::success(Redirect::to("/"), "Post successfully added.")
}

#[post("/<id>", data = "<post_form>")]
async fn update(
    conn: Connection<'_, Db>,
    id: i32,
    post_form: Form<post::Model>,
) -> Flash<Redirect> {
    let db = conn.into_inner();

    let form = post_form.into_inner();

    Mutation::update_post_by_id(db, id, form)
        .await
        .expect("could not update post");

    Flash::success(Redirect::to("/"), "Post successfully edited.")
}

#[get("/?<page>&<posts_per_page>")]
async fn list(
    conn: Connection<'_, Db>,
    page: Option<u64>,
    posts_per_page: Option<u64>,
    flash: Option<FlashMessage<'_>>,
) -> Template {
    let db = conn.into_inner();

    // Set page number and items per page
    let page = page.unwrap_or(1);
    let posts_per_page = posts_per_page.unwrap_or(DEFAULT_POSTS_PER_PAGE);
    if page == 0 {
        panic!("Page number cannot be zero");
    }

    let (posts, num_pages) = Query::find_posts_in_page(db, page, posts_per_page)
        .await
        .expect("Cannot find posts in page");

    Template::render(
        "index",
        json! ({
            "page": page,
            "posts_per_page": posts_per_page,
            "num_pages": num_pages,
            "posts": posts,
            "flash": flash.map(FlashMessage::into_inner),
        }),
    )
}

#[get("/<id>")]
async fn edit(conn: Connection<'_, Db>, id: i32) -> Template {
    let db = conn.into_inner();

    let post: Option<post::Model> = Query::find_post_by_id(db, id)
        .await
        .expect("could not find post");

    Template::render(
        "edit",
        json! ({
            "post": post,
        }),
    )
}

#[delete("/<id>")]
async fn delete(conn: Connection<'_, Db>, id: i32) -> Flash<Redirect> {
    let db = conn.into_inner();

    Mutation::delete_post(db, id)
        .await
        .expect("could not delete post");

    Flash::success(Redirect::to("/"), "Post successfully deleted.")
}

#[delete("/")]
async fn destroy(conn: Connection<'_, Db>) -> Result<(), rocket::response::Debug<String>> {
    let db = conn.into_inner();

    Mutation::delete_all_posts(db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[catch(404)]
pub fn not_found(req: &Request<'_>) -> Template {
    Template::render(
        "error/404",
        json! ({
            "uri": req.uri()
        }),
    )
}
*/

pub struct User {
    pub id: i32,
    pub email: String,
    pub password: String,
}

#[derive(FromForm)]
struct LoginReq {
    email: String,
    password_hash: String,
}
#[post("/login", data = "<user>")]
async fn login_user_handler(
    conn: Connection<'_, Db>,
    user: Form<LoginReq>,
) -> Result<Json<NetworkResponse>, Json<NetworkResponse>> {
    let form = user.into_inner();
    let email: String = form.email;
    let password: String = utils::encrypt_password(form.password_hash);
    match jwtauth::jwt::login_user(conn.into_inner(), &email, &password).await {
        Ok(token) => Ok(Json(NetworkResponse::Ok(token))),
        Err(network_response) => Err(Json(network_response)),
    }
}
#[derive(FromForm)]
struct UploadFileForm<'f> {
    file: TempFile<'f>,
}

#[post("/uploadFile", data = "<file_input>")]
async fn upload_to_telegram_handler(
    conn: Connection<'_, Db>,
    bot: &State<GlobalState>,
    mut file_input: Form<UploadFileForm<'_>>,
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
            let bot_guard = bot.bot.lock().await;
            let bot = bot_guard.deref();
            let mut file_name =
                std::env::var("UPLOAD_DIR").unwrap() + file_input.file.name().unwrap();
            file_input.file.persist_to(&file_name).await.unwrap();
            let _ = service::uploader(
                conn.into_inner(),
                bot,
                file_name.clone(),
                c.subject_id as u64,
                file_name,
            )
            .await;
            Ok(Json(NetworkResponse::Ok("Andato".to_string())))
        }
        Err(err_response) => Err(err_response),
    };

    response
}

#[get("/test/<id>")]
async fn testing_download(conn: Connection<'_, Db>, bot: &State<GlobalState>, id: u64) {
    let bot_guard = bot.bot.lock().await;
    let bot = bot_guard.deref();
    service::downloader(conn.into_inner(), bot, id).await;
}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    let conn = &Db::fetch(&rocket).unwrap().conn;
    let _ = migration::Migrator::up(conn, None).await;
    Ok(rocket)
}

struct GlobalState {
    bot: Mutex<Bot>,
}

#[tokio::main]
async fn start() -> Result<(), rocket::Error> {
    dotenv().ok(); // Loads the environment

    rocket::build()
        .attach(Db::init())
        .attach(AdHoc::try_on_ignite("Migrations", run_migrations))
        .mount("/", routes![root, login_user_handler, testing_download])
        .mount("/uploadTempTest", routes![upload_to_telegram_handler])
        .manage(GlobalState {
            bot: Mutex::from(Bot::from_env()),
        })
        .launch()
        .await
        .map(|_| ())
}

pub fn main() {
    let result = start();

    println!("Rocket: deorbit.");

    if let Some(err) = result.err() {
        println!("Error: {err}");
    }
}
