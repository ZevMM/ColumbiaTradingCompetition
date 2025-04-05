/// Defines all main message types for internal actor communication
use actix::*;
use serde::Serialize;
use std::sync::Arc;

use crate::{api_messages::OutgoingMessage, config::TraderIp};

#[derive(Message)]
#[rtype(result = "()")]
pub struct OpenMessage{
    pub ip: TraderIp,
    pub addr: Recipient<Arc<OutgoingMessage>>
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct CloseMessage{
    pub ip: TraderIp,
    pub addr: Recipient<Arc<OutgoingMessage>>
}

//not technically internal, but shouldn't be exposed to general users
#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub struct GameStartedMessage(pub String);


// Add this new message type
#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub struct GameEndMessage;