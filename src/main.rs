use actix::prelude::{
    Actor, ActorContext, Addr, AsyncContext, Handler, Message, StreamHandler,
};
use actix_web::{middleware, web, App, HttpRequest, HttpServer, Responder};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type SignalServerStateData = web::Data<Mutex<SignalServerState>>;

async fn index(
    state: SignalServerStateData,
    request: HttpRequest,
    stream: web::Payload,
) -> impl Responder {
    let mut resolved_server_state = state.lock().unwrap();
    ws::start(
        SignalSocket::new("asdf".to_owned(), &mut resolved_server_state),
        &request,
        stream,
    )
}

struct SignalSocket {
    user_name: String,
    another_sockets: Arc<Mutex<HashMap<String, Addr<SignalSocket>>>>,
}

impl SignalSocket {
    fn new(user_name: String, server_state: &mut SignalServerState) -> Self {
        let new_signal_socket = SignalSocket {
            user_name,
            another_sockets: server_state.sockets.clone(),
        };

        return new_signal_socket;
    }
}

impl Actor for SignalSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        let mut resolved_another_sockets = self.another_sockets.lock().unwrap();
        resolved_another_sockets.insert(self.user_name.clone(), context.address());
        println!("Signal Socket Opened")
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        let mut resolved_another_sockets = self.another_sockets.lock().unwrap();
        resolved_another_sockets.remove(&self.user_name);
        println!("Signal Socket Closed")
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SignalSocket {
    fn handle(
        &mut self,
        message: Result<ws::Message, ws::ProtocolError>,
        context: &mut Self::Context,
    ) {
        match message {
            Ok(ws::Message::Close(_)) => {
                println!("close request received. closing.");
                context.stop();
            }
            Ok(_) => {
                println!("some messaged received.");
            }
            Err(error) => eprintln!("error occurred during receive message: {}", error),
        }
    }
}

#[derive(Debug)]
enum MessageSendError {
    ParseError(serde_json::Error),
}

impl From<serde_json::Error> for MessageSendError {
    fn from(err: serde_json::Error) -> MessageSendError {
        MessageSendError::ParseError(err)
    }
}

impl std::fmt::Display for MessageSendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageSendError::ParseError(err) => write!(formatter, "{}", err),
        }
    }
}

impl std::error::Error for MessageSendError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            MessageSendError::ParseError(err) => Some(err),
        }
    }
}

impl Handler<Arc<Signal>> for SignalSocket {
    type Result = Result<(), MessageSendError>;

    fn handle(
        &mut self,
        message: Arc<Signal>,
        context: &mut Self::Context,
    ) -> Result<(), MessageSendError> {
        let resolved_message: &Signal = &message;
        context.text(serde_json::to_string(resolved_message)?);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
enum Signal {
    Offer(SessionDescriptionMessage),
    Answer(SessionDescriptionMessage),
    NewIceCandidate(IceCandidate),
}

#[derive(Serialize, Deserialize, Clone)]
struct SessionDescriptionMessage {
    target: String,
    name: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct IceCandidate {
    targe: String,
    candidate: String,
}

impl Signal {
    pub fn parse_json(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }
}

impl Message for Signal {
    type Result = Result<(), MessageSendError>;
}

struct SignalServerState {
    sockets: Arc<Mutex<HashMap<String, Addr<SignalSocket>>>>,
}

impl Default for SignalServerState {
    fn default() -> Self {
        SignalServerState {
            sockets: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(Mutex::new(SignalServerState::default()));
    HttpServer::new(move || {
        App::new()
            .register_data(state.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource("/signal").to(index))
    })
    .bind("0.0.0.0:8080")?
    .start()
    .await
}
