use poem_openapi::Object;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Object, Serialize, Deserialize)]
pub struct UserObject {
    pub pid: Uuid,
    pub id: i32,
    pub email: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Object, Serialize, Deserialize)]
pub struct SessionObject {
    pub session: Uuid,
    pub user: UserObject,
}
