/// main actor which manages connections and order flow data messages
/// transaction updates are sent from the orderbook add_order fn to this actor
/// this actor then fairly sends out transaction updates to all connected websockets.
// use actix_web::*;
use actix::*;
use std::sync::Arc;
use std::time::Duration;

use crate::api_messages::{JsonPayload, OutgoingMessage};
use crate::message_types::OpenMessage;

/// Maximum number of market-data events to accumulate before forcing a flush.
/// This caps latency under burst load.
const BATCH_MAX_SIZE: usize = 128;
/// Maximum time to wait before flushing a partial batch.
const BATCH_TIMEOUT: Duration = Duration::from_millis(1);

pub struct Server {
    connected_actors: Vec<Recipient<JsonPayload>>,
    /// Pending market-data events waiting to be broadcast.
    pending_batch: Vec<Arc<OutgoingMessage>>,
    /// Whether a timeout-based flush is already scheduled.
    flush_scheduled: bool,
}

impl Server {
    pub fn new() -> Server {
        warn!("Relay Server actor created");
        Server {
            // todo: capacity number should be abstracted to config file.
            connected_actors: Vec::with_capacity(1000),
            pending_batch: Vec::with_capacity(BATCH_MAX_SIZE),
            flush_scheduled: false,
        }
    }

    /// Serialize and broadcast the current batch to every connected client,
    /// then clear the batch.
    fn flush(&mut self) {
        self.flush_scheduled = false;
        if self.pending_batch.is_empty() {
            return;
        }

        // Build a JSON array containing all pending market-data events.
        // Each event is serialized once and shared as an Arc<str> for all clients.
        let refs: Vec<&OutgoingMessage> = self.pending_batch.iter().map(|m| &**m).collect();
        let payload = JsonPayload(Arc::from(serde_json::to_string(&refs).unwrap()));
        for connection in self.connected_actors.iter() {
            connection.do_send(payload.clone());
        }
        self.pending_batch.clear();
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<Arc<OutgoingMessage>> for Server {
    type Result = ();
    fn handle(&mut self, msg: Arc<OutgoingMessage>, ctx: &mut Self::Context) {
        self.pending_batch.push(msg);

        if self.pending_batch.len() >= BATCH_MAX_SIZE {
            // Batch is full: flush immediately to bound latency.
            self.flush();
        } else if self.connected_actors.len() <= 1 {
            // With only one client there is no fan-out savings from batching,
            // so send immediately to keep latency minimal.
            self.flush();
        } else if !self.flush_scheduled {
            // Schedule a timeout flush for any remaining partial batch.
            self.flush_scheduled = true;
            ctx.run_later(BATCH_TIMEOUT, |act, _ctx| act.flush());
        }
    }
}

impl Handler<OpenMessage> for Server {
    type Result = ();
    fn handle(&mut self, msg: OpenMessage, _ctx: &mut Self::Context) -> Self::Result {
        let res = self.connected_actors.push(msg.addr);
        debug!("New websocket actor registered: {:?}", &msg.ip);
        res
    }
}

impl Handler<crate::message_types::CloseMessage> for Server {
    type Result = ();
    fn handle(
        &mut self,
        msg: crate::message_types::CloseMessage,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let res = self.connected_actors.retain(|x| x != &msg.addr);
        debug!("Websocket actor disconnected: {:?}", &msg.ip);
        res
    }
}
