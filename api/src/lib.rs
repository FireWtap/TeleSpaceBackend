#[macro_use]
extern crate rocket;

use rocket::fairing::{self, AdHoc};
use rocket::fs::{relative, FileServer};
use rocket::response::{content, status};
use rocket::{Build, Response, Rocket};
use rocket::form::Form;
use rocket::response::status::NotFound;
use rocket::serde::json::Json;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, Cursor, DatabaseConnection, DbErr, EntityTrait, QueryFilter};

use migration::MigratorTrait;
use sea_orm_rocket::{Connection, Database};
use teloxide::Bot;
use teloxide::prelude::{Message, Requester};

mod pool;
mod jwtauth;
mod responses;
mod utils;

use pool::Db;

use entity::prelude::Users;
pub use entity::*;
use entity::users::Model;
use crate::jwtauth::jwt::create_jwt;
use crate::responses::{NetworkResponse, ResponseBody};
use crate::responses::ResponseBody::AuthToken;

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
async fn login_user(conn: Connection<'_, Db>, email: &String, password: &String)  -> Result<String, NetworkResponse>  {
    let db:&DatabaseConnection = conn.into_inner();
    let user = Users::find()
        .filter(users::Column::Email.eq(email))
        .filter(users::Column::PasswordHash.eq(password))
        .one(db)
        .await
        ;
    match user{
        Ok(Some(user)) => {
            let token = create_jwt(user.id).map_err(|err| {
                let response = responses::Response {
                    body: ResponseBody::Message(format!("JWT creation error: {}", err)),
                };
                NetworkResponse::BadRequest(serde_json::to_string(&response).unwrap())
            })?;
            Ok(token)
        }
        _ => {
            Err(NetworkResponse::NotFound("User not found or Wrong Password".to_string()))}
    }


}
#[derive(FromForm)]
struct LoginReq{
    email: String,
    password_hash: String
}
#[post("/login", data = "<user>")]
async fn login_user_handler(conn: Connection<'_, Db>, user: Form<LoginReq>) -> Result<Json<NetworkResponse>, Json<NetworkResponse>> {
    let form = user.into_inner();
    let email:String = form.email;
    let password:String = utils::encrypt_password(form.password_hash);
    match login_user(conn, &email, &password).await {
        Ok(token) => Ok(Json(NetworkResponse::Ok(token))),
        Err(network_response) => Err(Json(network_response)),
    }

}



async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    let conn = &Db::fetch(&rocket).unwrap().conn;
    let _ = migration::Migrator::up(conn, None).await;
    Ok(rocket)
}

#[tokio::main]
async fn start() -> Result<(), rocket::Error> {

    let bot = Bot::new("TOKEN HERE, WILL BE FROM ENV.".to_string());
    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        bot.send_dice(msg.chat.id).await?;
        Ok(())
    }).await;
    rocket::build()
        .attach(Db::init())
        .attach(AdHoc::try_on_ignite("Migrations", run_migrations))
        .mount("/", routes![root,login_user_handler])
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
