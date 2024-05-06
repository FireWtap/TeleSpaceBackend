use rocket::serde::Serialize;

#[derive(Debug, Serialize)]
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
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Response {
    pub body: ResponseBody,
}
