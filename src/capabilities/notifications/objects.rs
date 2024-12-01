use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationMailMessage {
    pub template: String,
    pub subject: String,
    pub name: String,
    pub email: String, 
    pub data: Option<serde_json::Value>  
}
