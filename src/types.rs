use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Version {
    NewChatRoom,
    Message,
    ConfigUpdate,
    SystemPing,
    ContestUpdate,
    AiReply,
    ResponseFromMcp,
    GetUserDetails,
    GetNextQuestion
}



#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum CommSender {
    System,
    User,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum MessageSender {
    Ai,
    User,
    System,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub id: String,
    pub text: String,
    pub sender: MessageSender,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_code: Option<bool>,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct UserConfig {
    pub explain_style: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommMessage {
    pub version: Version,
    pub sender: CommSender,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_apikey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<Message>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<UserConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_updated: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiResponse {
    _candidates: Vec<serde_json::Value>,
}

