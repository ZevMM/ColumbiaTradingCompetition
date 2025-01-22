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

// need to impl all "OutgoingOrderMessage"
// impl Handler<crate::orderbook::LimLevUpdate> for Server {
//     // forward limit level updates to all connected actors
//     type Result = ();
//     fn handle(
//         &mut self,
//         msg: crate::orderbook::LimLevUpdate,
//         ctx: &mut Self::Context,
//     ) -> Self::Result {
//         debug!("New LimLevUpdate Message Received by Relay Server");
//         let msg_arc = Arc::new(msg);
//         for connection in self.connected_actors.iter() {
//             // connection.do_send(msg_arc.clone());
//         }
//     }
// }


impl Handler<Arc<OutgoingMessage>> for Server {
    type Result = ();
    fn handle(&mut self, msg: Arc<OutgoingMessage>, ctx: &mut Self::Context) {      
        // there has to be a nicer way to do this, but cant figure out how to access inner type when doing a default match
        // ctx.text(serde_json::to_string(&msg.d).unwrap());

        // need to check that we are not sending out to trader which placed the order
        // to avoid double counting, could match on address, but seems bad. 

        // do not need to avoid double messages, just advise clients that they shouldn't update 
        // their orderbook on personal trade messages (i.e. they can deal with this)

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
        debug!("New websocket actor registered!");
        debug!("Full list: {:?}", &self.connected_actors);        
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
        debug!("Websocket actor disconnected!");
        debug!("Full list: {:?}", &self.connected_actors);        
        res
    }
}