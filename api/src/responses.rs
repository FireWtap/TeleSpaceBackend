use rocket::serde::Serialize;
use rocket::Responder;
use rocket::response::status;

#[derive( Debug, Serialize)]
pub enum NetworkResponse {

    Ok(String),

    Created(String),

    BadRequest(String),

    Unauthorized(String),
    NotFound(String),
    Conflict(String),
}

#[derive(Serialize)]
pub enum ResponseBody {
    Message(String),
    AuthToken(String),
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Response {
    pub body: ResponseBody,
}
