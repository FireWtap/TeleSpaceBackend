#[macro_use]
extern crate rocket;

use dotenvy::dotenv;

use rocket::fairing::{self, AdHoc};
use rocket::form::Form;
use rocket::fs::TempFile;

use rocket::response::content;
use rocket::serde::json::{json, Json};

use rocket::http::Method;
use rocket::{Build, Rocket, State};
use rocket_cors::{AllowedOrigins, CorsOptions};
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, InsertResult, QueryFilter, QueryOrder,
};
use std::path::Path;
use std::sync::Arc;

use sea_orm::prelude::Uuid;
use sea_orm_rocket::{Connection, Database, Initializer};
use serde::{Deserialize, Serialize};
use tracing::debug;

use migration::MigratorTrait;

use teloxide::Bot;
use tokio::sync::Mutex;

mod handlers;
mod jwtauth;
mod pool;
mod responses;
mod utils;

use pool::Db;

use crate::jwtauth::jwt::JWT;

use crate::responses::NetworkResponse;

use entity::prelude::{Files, TaskList};

use crate::handlers::dir_handlers::{
    delete_directory_handler, get_directory_name_handler, get_parent_directory, list_directory,
    new_dir_handler,
};
use crate::handlers::file_handlers::{
    clear_cache_handler, delete_file_handler, file_info_handler, locally_stored_download_handler,
};

use crate::handlers::task_handlers::get_all_tasks;

pub use entity::*;
use service::task_queue;
use service::task_queue::{TaskQueue, TaskType};
use service::worker::worker;

#[get("/")]
async fn root() -> content::RawJson<String> {
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
    dir: i32,
    filename: String, //Needed because rocket deletes the extention of the file in TempFile
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
            let db = conn.into_inner();
            let file_name = std::env::var("UPLOAD_DIR").unwrap() + file_input.filename.as_str();

            // Modifica: Gestisce dir = -1 come None
            let original_dir = if file_input.dir == -1 {
                None
            } else {
                Some(file_input.dir)
            };

            let dir = match original_dir {
                Some(dir_id) => files::Entity::find_by_id(dir_id)
                    .filter(files::Column::Type.eq(true))
                    .one(&db.clone())
                    .await
                    .unwrap(),
                None => None,
            };

            println!("{:#?}", dir);
            let valid_dir = match dir {
                Some(d) => true,                        // La directory esiste ed è valida
                None if original_dir.is_none() => true, // dir era -1 o non specificata, usiamo la root directory
                None => false,                          // dir era specificata ma non valida
            };

            if valid_dir {
                file_input.file.persist_to(&file_name).await.unwrap();
                /*let x =
                match tokio::fs::copy(&file_input.file.path().unwrap(), &file_name ).await{
                    Ok(_) => tokio::fs::remove_file(&file_input.file.path().unwrap()).await,
                    Err(e) => {
                        Err(std::io::Error::new(std::io::ErrorKind::Other, "Error"))
                    }
                };*/

                let file_opened = Path::new(&file_name);
                let file_size = file_opened.metadata().unwrap().len();
                let file = files::ActiveModel {
                    id: Default::default(),
                    filename: Set(file_name.clone()),
                    r#type: Set(false),
                    original_size: Set(file_size as i32),
                    user: Set(c.subject_id),
                    upload_time: Default::default(),
                    last_download: Set(None),
                    locally_stored: Set(None),
                    parent_dir: Set(original_dir.map(|dir| dir as i32)), // Converte Some(-1) o qualsiasi Some(dir_id) in Some(dir_id as i32), None rimane None
                };
                let res: InsertResult<files::ActiveModel> =
                    files::Entity::insert(file).exec(db).await.unwrap();
                let file_id = res.last_insert_id;
                let task_uuid = Uuid::new_v4();
                let upload_task = task_queue::TaskType::Upload {
                    id: task_uuid,
                    file_path: file_name.clone(),
                    user_id: c.subject_id as u64,
                    file_name: file_name,
                    file_id: file_id as u64,
                };

                task_queue.add_task(upload_task).await.unwrap();
                Ok(Json(NetworkResponse::Ok(task_uuid.to_string())))
            } else {
                Err(Json(NetworkResponse::NotFound(
                    "Invalid folder".to_string(),
                )))
            }
        }
        Err(err_response) => Err(err_response),
    };

    response
}

#[get("/listAllFiles")]
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
                .filter(files::Column::User.eq(c.subject_id as i32))
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

#[get("/downloadFile/<id>")]
async fn download_file_handler(
    conn: Connection<'_, Db>,
    state: &State<GlobalState>,
    id: u64,
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
            //service::downloader(&conn.into_inner(), bot, id, Uuid::new_v4()).await;
            //check if file is locally stored
            let file_info = Files::find_by_id(id as i32).one(db).await.unwrap().unwrap();
            if !file_info.locally_stored.is_none() && file_info.locally_stored.unwrap().eq(&true) {
                return Err(Json::from(NetworkResponse::Ok(
                    "File already stored locally, just download it".to_string(),
                )));
            }
            //check if the file has been successfully downloaded though looking the task list and seeing if the last job for it has been completed
            let related_task = TaskList::find()
                .filter(task_list::Column::File.eq(id))
                .filter(task_list::Column::Type.eq(1))
                .order_by_asc(task_list::Column::CompletionTime)
                .one(db)
                .await
                .unwrap();

            match related_task {
                None => Err(Json::from(NetworkResponse::NotFound(
                    "File not found or yet to be uploaded".to_string(),
                ))),
                Some(_) => {
                    //File has been successfully uploaded to telegram, which means we can download it locally and return an UUID to retrieve the download link
                    //put a new task in list and return the uuid of the new task that will be a download task
                    let task_uuid = Uuid::new_v4();
                    let task = TaskType::Download {
                        id: task_uuid,
                        db_file_id: id,
                        user_id: c.subject_id as u64,
                    };
                    //Get the queue
                    let task_queue = &state.queue;
                    task_queue.add_task(task).await.unwrap();
                    Ok(Json(NetworkResponse::Ok(String::from(task_uuid))))
                }
            }
        }
        Err(e) => Err(e),
    };
    response
}
#[derive(Serialize, Deserialize)]
struct StatusResponse {
    uuid: String,
    status: String,
    resource: Option<u64>,
    r#type: bool,
}
#[get("/getStatus/<uuid>")]
pub async fn get_status_handler(
    conn: Connection<'_, Db>,
    uuid: &str,
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
            let task = TaskList::find()
                .find_with_related(Files)
                .filter(<entity::prelude::TaskList as EntityTrait>::Column::Id.eq(uuid))
                .all(db)
                .await
                .unwrap();
            if task.is_empty() || task[0].1.is_empty() {
                return Err(Json(NetworkResponse::NotFound(
                    "Task or file not found".to_string(),
                )));
            }
            let related_task = &task[0].0;
            let related_file = &task[0].1[0];
            let user_id = c.subject_id;

            if user_id.eq(&related_file.user) {
                //User is authorized
                let mut ok_response = StatusResponse {
                    uuid: uuid.to_string(),
                    r#type: related_task.r#type,
                    status: related_task.status.clone(),
                    resource: None,
                };
                if (related_task.status.eq("COMPLETED")) {
                    ok_response.resource = Option::from(related_file.id as u64);
                }

                Ok(Json(NetworkResponse::Ok(json!(ok_response).to_string())))
            } else {
                Err(Json(NetworkResponse::Unauthorized(
                    "Unauthorized user for this file.".to_string(),
                )))
            }
        }
        Err(e) => Err(e),
    };
    response
    //check if there is a task with that uuid and if the file is correlated to the user retrieving the status
    //if it exists:
    //completed: return link to downloadable file or whatever if the task is a download one
    //working/waiting, just status
    //completed but upload: just status
}

#[get("/me")]
async fn get_me_handler(
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
            let sub_r = json!(c).to_string();
            Ok(Json(NetworkResponse::Ok(sub_r)))
        }
        Err(e) => Err(e),
    };
    response
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
    let worker_connection: DatabaseConnection =
        sea_orm::Database::connect("sqlite://db.sqlite?mode=rwc")
            .await
            .unwrap();
    let worker = tokio::spawn(async move {
        worker(receiver, Arc::new(worker_connection), worker_bot).await;
    });

    debug!("Setting up CORS...");
    let allowed_origins = AllowedOrigins::some_exact(&[
        "http://localhost:5173", // Aggiungi qui altri domini se necessario
    ]);
    let allowed_origins = AllowedOrigins::all();
    let cors = CorsOptions {
        allowed_origins,
        allowed_methods: vec![
            rocket::http::Method::Get,
            rocket::http::Method::Post,
            Method::Delete,
        ]
        .into_iter()
        .map(From::from)
        .collect(),
        allowed_headers: rocket_cors::AllowedHeaders::some(&[
            "Authorization",
            "Accept",
            "Content-Type",
        ]),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .expect("CORS configuration failed");

    debug!("Firing up rocket");
    rocket::build()
        .attach(db)
        .attach(AdHoc::try_on_ignite("Migrations", run_migrations))
        .attach(cors)
        .mount(
            "/",
            routes![
                root,
                login_user_handler,
                download_file_handler,
                list_all,
                upload_to_telegram_handler,
                get_status_handler,
                get_me_handler,
                new_dir_handler,
                list_directory,
                get_directory_name_handler,
                get_parent_directory,
                delete_directory_handler,
                delete_file_handler,
                locally_stored_download_handler,
                file_info_handler,
                clear_cache_handler,
                get_all_tasks
            ],
        )
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
