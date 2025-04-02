/// main actor which manages connections and order flow data messages
/// transaction updates are sent from the orderbook add_order fn to this actor
/// this actor then fairly sends out transaction updates to all connected websockets.
// use actix_web::*;
use actix::*;
use std::sync::Arc;

use crate::{api_messages::OutgoingMessage, message_types::OpenMessage};

pub struct Server{
    connected_actors: Vec<Recipient<Arc<OutgoingMessage>>>,
}

impl Server {
    pub fn new() -> Server {
        warn!("Relay Server actor created");
        Server {
            // todo: capacity number should be abstracted to config file. 
            connected_actors: Vec::with_capacity(1000)
        }
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<Arc<OutgoingMessage>> for Server {
    type Result = ();
    fn handle(&mut self, msg: Arc<OutgoingMessage>, ctx: &mut Self::Context) {      
        match *msg {
            OutgoingMessage::NewRestingOrderMessage(m) => {
                let msg_arc = Arc::new(OutgoingMessage::NewRestingOrderMessage(m));
                for connection in self.connected_actors.iter() {
                    connection.do_send(msg_arc.clone());
                }
            }
            OutgoingMessage::TradeOccurredMessage(m) =>  {
                let msg_arc = Arc::new(OutgoingMessage::TradeOccurredMessage(m));
                for connection in self.connected_actors.iter() {
                    connection.do_send(msg_arc.clone());
                }
            }
            OutgoingMessage::CancelOccurredMessage(m) => {
                let msg_arc = Arc::new(OutgoingMessage::CancelOccurredMessage(m));
                for connection in self.connected_actors.iter() {
                    connection.do_send(msg_arc.clone());
                }
            }
            // This should never be reached, quick hack before first test
            _ => todo!(),            
        }
    }
}

impl Handler<crate::message_types::OpenMessage> for Server{
    type Result = ();
    fn handle(
        &mut self,
        msg: OpenMessage,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let res = self.connected_actors.push(msg.addr);
        debug!("New websocket actor registered: {:?}", &msg.ip);       
        res
    }
}

impl Handler<crate::message_types::CloseMessage> for Server{
    type Result = ();
    fn handle(
        &mut self,
        msg: crate::message_types::CloseMessage,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let res = self.connected_actors.retain(|x| x != &msg.addr);
        debug!("Websocket actor disconnected: {:?}", &msg.ip);        
        res
    }
}