use poem::Error;
use poem_openapi::payload::Json;
use poem_openapi::types::{ParseFromJSON, ToJSON};
use poem_openapi::{ApiResponse, Object};
use serde::{Deserialize, Serialize};

use crate::capabilities::logger;

use super::common_error::CommonError;

/// Common Response for web server with status codes
#[derive(ApiResponse)]
#[oai(bad_request_handler = "bad_request_handler")]
pub enum CommonResponse<T: Serialize + ParseFromJSON + ToJSON + Send + Sync> {
    /// Repsonse 200 without
    #[oai(status = 200)]
    Done,
    /// Repsonse 201 with JSON data
    #[oai(status = 201)]
    Ok(Json<Data<T>>),
    /// Repsonse 201 with ApiError
    #[oai(status = 202)]
    Err(Json<CommonError>),
    /// Repsonse 400
    #[oai(status = 400)]
    BadRequest,
    /// Repsonse 401
    #[oai(status = 401)]
    Unauthorized,
    /// Repsonse 403
    #[oai(status = 403)]
    Forbidden,
    /// Repsonse 404
    #[oai(status = 404)]
    NotFound,
    /// Repsonse 409
    #[oai(status = 409)]
    Conflict,
    /// Repsonse 500
    #[oai(status = 500)]
    InternalServerError,
}

fn bad_request_handler<T: Serialize + ToJSON + ParseFromJSON>(err: Error) -> CommonResponse<T> {
    logger::debug(&format!("{}", err));
    CommonResponse::BadRequest
}

#[derive(Debug, Object, Deserialize)]
pub struct Data<T: Serialize + ToJSON + ParseFromJSON> {
    pub data: T,
}

impl<T: Serialize + ToJSON + ParseFromJSON> Data<T> {
    pub fn from(data: T) -> Self {
        Self { data }
    }
}

pub fn ok<T: Serialize + ToJSON + ParseFromJSON>(data: T) -> CommonResponse<T> {
    CommonResponse::Ok(Json(Data::from(data)))
}

pub fn err<T: Serialize + ToJSON + ParseFromJSON>(error: CommonError) -> CommonResponse<T> {
    CommonResponse::Err(Json(error))
}
