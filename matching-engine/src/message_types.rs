/// Defines all main message types for internal actor communication
use actix::*;
use serde::Serialize;
use std::sync::Arc;

use crate::{api_messages::OutgoingMessage, config::TraderIp, orderbook::{TraderId}};

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



#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub struct GameStartedMessage(pub String);


// Add this new message type
#[derive(Message, Debug, Serialize, Clone)]
#[rtype(result = "()")]
pub struct GameEndMessage {
    pub final_score: usize,
}