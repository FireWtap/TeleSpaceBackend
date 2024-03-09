use crate::responses;
use crate::responses::{NetworkResponse, Response, ResponseBody};
use chrono::Utc;
use dotenvy::dotenv;
use entity::prelude::Users;
use entity::users;
use jsonwebtoken::errors::{Error, ErrorKind};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::Status;
use rocket::request::Outcome;
use rocket::request::{self, FromRequest, Request};
use rocket::serde::{Deserialize, Serialize};
use sea_orm::{ColumnTrait, QueryFilter};
use sea_orm::{DatabaseConnection, EntityTrait};

use std::env;

#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    pub subject_id: i32,
    pub email: String,
    exp: usize,
}

#[derive(Debug)]
pub struct JWT {
    pub claims: Claims,
}

pub fn create_jwt(id: i32, user_email: String) -> Result<String, Error> {
    dotenv().ok();
    let secret = std::env::var("JWT_SECRET").unwrap();

    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(24 * 60 * 60))
        .expect("Invalid timestamp")
        .timestamp();

    // 👇 New!
    let claims = Claims {
        subject_id: id,
        exp: expiration as usize,
        email: user_email,
    };

    // 👇 New!
    let header = Header::new(Algorithm::HS512);

    encode(
        &header,
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn decode_jwt(token: String) -> Result<Claims, ErrorKind> {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set.");
    let token = token.trim_start_matches("Bearer").trim();
    match decode(
        &token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS512),
    ) {
        Ok(token) => Ok(token.claims),
        Err(err) => Err(err.kind().to_owned()),
    }
}
#[rocket::async_trait]
impl<'r> FromRequest<'r> for JWT {
    type Error = NetworkResponse;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        fn is_valid(key: &str) -> Result<Claims, Error> {
            Ok(decode_jwt(String::from(key))?)
        }

        match req.headers().get_one("authorization") {
            None => {
                let response = Response {
                    body: ResponseBody::Message(String::from(
                        "Error validating JWT token - No token provided",
                    )),
                };
                Outcome::Error((
                    Status::Unauthorized,
                    NetworkResponse::Unauthorized(serde_json::to_string(&response).unwrap()),
                ))
            }
            Some(key) => match is_valid(key) {
                Ok(claims) => Outcome::Success(JWT { claims }),
                Err(err) => match &err.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                        let response = Response {
                            body: ResponseBody::Message(
                                "Error validating JWT token - Expired Token".to_string(),
                            ),
                        };
                        Outcome::Error((
                            Status::Unauthorized,
                            NetworkResponse::Unauthorized(
                                serde_json::to_string(&response).unwrap(),
                            ),
                        ))
                    }
                    jsonwebtoken::errors::ErrorKind::InvalidToken => {
                        let response = Response {
                            body: ResponseBody::Message(
                                "Error validating JWT token - Invalid Token".to_string(),
                            ),
                        };
                        Outcome::Error((
                            Status::Unauthorized,
                            NetworkResponse::Unauthorized(
                                serde_json::to_string(&response).unwrap(),
                            ),
                        ))
                    }
                    _ => {
                        let response = Response {
                            body: ResponseBody::Message(format!(
                                "Error validating JWT token - {}",
                                err
                            )),
                        };
                        Outcome::Error((
                            Status::Unauthorized,
                            NetworkResponse::Unauthorized(
                                serde_json::to_string(&response).unwrap(),
                            ),
                        ))
                    }
                },
            },
        }
    }
}

pub async fn login_user(
    db: &DatabaseConnection,
    email: &String,
    password: &String,
) -> Result<String, NetworkResponse> {
    let user = Users::find()
        .filter(users::Column::Email.eq(email))
        .filter(users::Column::PasswordHash.eq(password))
        .one(db)
        .await;
    match user {
        Ok(Some(user)) => {
            let token = create_jwt(user.id, user.email).map_err(|err| {
                let response = responses::Response {
                    body: ResponseBody::Message(format!("JWT creation error: {}", err)),
                };
                NetworkResponse::BadRequest(serde_json::to_string(&response).unwrap())
            })?;
            Ok(token)
        }
        _ => Err(NetworkResponse::NotFound(
            "User not found or Wrong Password".to_string(),
        )),
    }
}
