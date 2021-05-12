use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Output {
    #[serde(rename = "user-joined")]
    UserJoined(UserJoinedOutput),
    #[serde(rename = "user-disconnect")]
    UserDisconnect(UserDiscconnectOutput),
    #[serde(rename = "user-message")]
    UserMessage(UserMessageOutput),
    #[serde(rename = "message")]
    Message(UserMessageOutput),
    #[serde(rename = "error")]
    Error(ErrorOutput),
    #[serde(rename = "keep-alive-tick")]
    KeepAliveTick,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum Input {
    #[serde(rename = "join")]
    Join(JoinEvent),
    #[serde(rename = "message")]
    Message(MessageEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "code")]
pub enum ErrorOutput {
    #[serde(rename = "invalid-session")]
    InvalidSession,
    #[serde(rename = "invalid-message-request")]
    InvalidMessageRequest,
    #[serde(rename = "channel-full")]
    ChannelFull,
    #[serde(rename = "name-taken")]
    NameTaken,
}

#[derive(Debug, Clone)]
pub struct RequestPacket {
    pub session_id: Uuid,
    pub channel_id: Uuid,
    pub body: Input,
}
 
impl RequestPacket {
    pub fn new(session_id: Uuid, channel_id: Uuid, body: Input) -> Self {
        RequestPacket {
            session_id,
            channel_id,
            body,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResponsePacket {
    pub session_id: Uuid,
    pub channel_id: Uuid,
    pub output: Output,
}

impl ResponsePacket {
    pub fn new(session_id: Uuid, channel_id: Uuid, output: Output) -> Self {
        ResponsePacket {
            session_id,
            channel_id,
            output,
        }
    }
}

// MODEL JSON IMPLMENETATION
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserModelResponse {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelModelResponse {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageModelResponse {
    pub id: Uuid,
    pub body: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

// INCOMING EVENTS
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinEvent {
    pub user_name: String,
    // TODO: ho do we get JSON into Uuid format?
    // pub user_id: Uuid,
    // pub channel_id: Uuid
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageEvent {
    pub body: String,
}

// OUTGOING EVENTS

// Generated anytime a user joins a channel
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserJoinedOutput {
    pub channels: Vec<ChannelModelResponse>,
    pub user: UserModelResponse,
}

impl UserJoinedOutput {
    pub fn new(
        channels: Vec<ChannelModelResponse>,
        user: UserModelResponse,
    ) -> Self {
        UserJoinedOutput {
            channels,
            user,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDiscconnectOutput {
    pub user_id: Uuid,
}

impl UserDiscconnectOutput {
    pub fn new(user_id: Uuid) -> Self {
        UserDiscconnectOutput { user_id }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMessageOutput {
    pub message: MessageModelResponse,
    pub channel: ChannelModelResponse,
}

impl UserMessageOutput {
    pub fn new(message: MessageModelResponse, channel: ChannelModelResponse) -> Self {
        UserMessageOutput { message, channel }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEvent {}
