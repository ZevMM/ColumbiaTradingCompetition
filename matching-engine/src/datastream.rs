use actix::prelude::*;
use actix_web::Error;
use actix_web_actors::ws;
use log::info;
use std::sync::Arc;
use std::time::{Duration, Instant};
use crate::api_messages::OutgoingMessage;
use crate::message_types::{CloseMessage, OpenMessage};
use actix_web::{web, HttpRequest, HttpResponse};
extern crate env_logger;
use crate::GlobalState;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(4);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Read-only market data WebSocket actor

pub struct MarketDataActor {
    hb: Instant,
    global_state: web::Data<GlobalState>,
    relay_server_addr: web::Data<Addr<crate::connection_server::Server>>,
}

impl Actor for MarketDataActor {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Send initial orderbook state
        let orderbooks = &self.global_state.global_orderbook_state;
        if let Ok(book_json) = serde_json::to_string(&orderbooks) {
            ctx.text(format!("{{\"GameState\" : {}}}", book_json));
        } else {
            error!("Failed to serialize initial orderbook state");
        }

        // Register for market data updates
        self.relay_server_addr.do_send(OpenMessage {
            ip: "0.0.0.0".parse().unwrap(),
            addr: ctx.address().recipient(),
        });
        
        self.hb(ctx);
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        self.relay_server_addr.do_send(CloseMessage {
            ip: "0.0.0.0".parse().unwrap(),
            addr: ctx.address().recipient(),
        });
    }
}

pub async fn market_data_websocket(
    req: HttpRequest,
    stream: web::Payload,
    relay_server_addr: web::Data<Addr<crate::connection_server::Server>>,
    global_state: web::Data<GlobalState>,
) -> Result<HttpResponse, Error> {
    info!("New market data websocket connection");
    
    ws::start(
        MarketDataActor {
            hb: Instant::now(),
            global_state: global_state.clone(),
            relay_server_addr: relay_server_addr.clone(),
        },
        &req,
        stream,
    )
}

// Add after MarketDataActor implementation
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MarketDataActor {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(_)) => {
                // Ignore text messages - read only connection
                ctx.text(r#"{"error": "This is a read-only market data connection"}"#);
            }
            Ok(ws::Message::Close(reason)) => {
                info!("Market data connection closing");
                ctx.close(reason);
                ctx.stop();
            }
            _ => {
                error!("Unexpected message type on market data connection");
                ctx.stop();
            }
        }
    }
}

// Handle market data messages
impl Handler<Arc<OutgoingMessage>> for MarketDataActor {
    type Result = ();
    
    fn handle(&mut self, msg: Arc<OutgoingMessage>, ctx: &mut Self::Context) {
        // Forward all market data messages
        match *msg {
            OutgoingMessage::NewRestingOrderMessage(_) |
            OutgoingMessage::TradeOccurredMessage(_) |
            OutgoingMessage::CancelOccurredMessage(_) => {
                if let Ok(json) = serde_json::to_string(&*msg) {
                    ctx.text(json);
                }
            }
            _ => {} // Ignore other message types
        }
    }
}

// Handle heartbeat checks
impl MarketDataActor {
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                warn!("Market data client timed out");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}