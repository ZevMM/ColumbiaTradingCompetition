/// Defines all main message types for internal actor communication
use actix::*;
use serde::Serialize;

use crate::api_messages::JsonPayload;
use crate::config::TraderIp;

#[derive(Message)]
#[rtype(result = "()")]
pub struct OpenMessage{
    pub ip: TraderIp,
    pub addr: Recipient<JsonPayload>
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct CloseMessage{
    pub ip: TraderIp,
    pub addr: Recipient<JsonPayload>
}

//not technically internal, but shouldn't be exposed to general users
#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub struct GameStartedMessage(pub String);


// Add this new message type
#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub struct GameEndMessage;

#[derive(Message)]
#[rtype(result = "()")]
pub struct KickMessage;
