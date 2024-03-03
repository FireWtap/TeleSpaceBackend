#[macro_use]
extern crate rocket;

use std::env;
use dotenvy::dotenv;

use rocket::fairing::{self, AdHoc};
use rocket::form::Form;
use rocket::fs::TempFile;

use rocket::response::content;
use rocket::serde::json::{json, Json};

use rocket::{Build, Rocket, State};
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, EntityTrait, InsertResult, QueryOrder};
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use rocket::yansi::Paint;

use sea_orm::prelude::Uuid;
use sea_orm_rocket::{Connection, Database, Initializer};
use tracing::debug;

use migration::MigratorTrait;

use teloxide::Bot;
use tokio::sync::Mutex;

mod jwtauth;
mod pool;
mod responses;
mod utils;

use pool::Db;

use crate::jwtauth::jwt::JWT;

use crate::responses::NetworkResponse;

use entity::prelude::{Files, Users};

pub use entity::*;
use service::task_queue;

use service::task_queue::TaskType::Upload;
use service::task_queue::{TaskQueue, TaskType};
use service::worker::worker;

const DEFAULT_POSTS_PER_PAGE: u64 = 5;

#[get("/")]
async fn root(conn: Connection<'_, Db>) -> content::RawJson<String> {
    content::RawJson(format!("users:'{:?}'", "testing stuff"))
}

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
    state: &State<GlobalState>,
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
            let task_queue = &state.queue;

            let file_name = std::env::var("UPLOAD_DIR").unwrap() + file_input.file.name().unwrap();

            //Persist to upload_dir
            file_input.file.persist_to(&file_name).await.unwrap();
            //Insert the file into db and start the upload to telegram through inserting the task into queue
            let file_opened = Path::new(&file_name);
            let file_size = file_opened.metadata().unwrap().len();
            let file = files::ActiveModel {
                id: Default::default(),
                filename: Set(file_name.clone()),
                r#type: Set(false),
                original_size: Set(file_size as i32),
                user: Set(c.subject_id),
                upload_time: Default::default()
            };
            //Adding file row to db
            let res: InsertResult<files::ActiveModel> =
                entity::files::Entity::insert(file).exec(conn.into_inner()).await.unwrap();
            let file_id: i32 = res.last_insert_id;
            //Creating task
            let task_uuid = Uuid::new_v4();
            let upload_task = task_queue::TaskType::Upload {
                id: task_uuid,
                file_path: file_name.clone(),
                user_id: c.subject_id as u64,
                file_name: file_name.clone(),
                file_id: file_id as u64
            };

            task_queue.add_task(upload_task).await.unwrap();
            //Returns uuid of the task. Can be used to query and check if the file has been correctly uploaded to telegram or not
            Ok(Json(NetworkResponse::Ok(String::from(task_uuid))))
        }
        Err(err_response) => Err(err_response),
    };

    response
}

#[get("/listAll")]
async fn list_all(
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
            let _user_id = c.subject_id;
            let db = conn.into_inner();
            let query_all = Files::find()
                .order_by_asc(files::Column::Type)
                .order_by_asc(files::Column::Filename)
                .all(db)
                .await
                .unwrap();
            Ok(Json(NetworkResponse::Ok(json!(query_all).to_string())))
        }
        Err(e) => Err(e),
    };
    response
}

#[get("/test/<id>")]
async fn testing_download(conn: Connection<'_, Db>, bot: &State<GlobalState>, id: u64) {
    let bot_guard = bot.bot.lock().await;
    let bot = bot_guard.deref();
    service::downloader(conn.into_inner(), bot, id, Uuid::new_v4()).await;
}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    let conn = &Db::fetch(&rocket).unwrap().conn;
    let _ = migration::Migrator::up(conn, None).await;
    Ok(rocket)
}

struct GlobalState {
    bot: Mutex<Bot>,
    queue: TaskQueue,
}

#[tokio::main]
async fn start() -> Result<(), rocket::Error> {
    debug!("Loading up .env...");
    dotenv().ok(); // Loads the environment
    debug!("Creating the task queue...");
    let (task_queue, receiver) = TaskQueue::new().await;
    debug!("Init database...");
    let db: Initializer<Db> = Db::init();
    debug!("Init bot connection...");
    let bot = Bot::from_env();


    let worker_bot = Bot::from_env();
    let worker_connection: DatabaseConnection = sea_orm::Database::connect("sqlite://db.sqlite?mode=rwc").await.unwrap();
    let worker = tokio::spawn(async move {
        worker(receiver, Arc::new(worker_connection), worker_bot).await;
    });

    debug!("Firing up rocket");
    rocket::build()
        .attach(db)
        .attach(AdHoc::try_on_ignite("Migrations", run_migrations))
        .mount(
            "/",
            routes![root, login_user_handler, testing_download, list_all],
        )
        .mount("/uploadTempTest", routes![upload_to_telegram_handler])
        .manage(GlobalState {
            bot: Mutex::from(bot.clone()),
            queue: task_queue,
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
