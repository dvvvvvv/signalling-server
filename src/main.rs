use actix::prelude::{
    Actor, ActorContext, AsyncContext, Handler, Message, Recipient, StreamHandler,
};
use actix_web::{middleware, web, App, HttpRequest, HttpServer, Responder};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type SignalServerStateData = web::Data<Mutex<SignalServerState>>;

async fn index(
    state: SignalServerStateData,
    path: web::Path<u32>,
    request: HttpRequest,
    stream: web::Payload,
) -> impl Responder {
    let channel = state
        .lock()
        .unwrap()
        .channels
        .entry(*path)
        .or_insert_with(|| Arc::new(Mutex::new(SignalChannel::default())))
        .clone();
    state.lock().unwrap().count += 1;
    ws::start(SignalSocket::new(channel), &request, stream)
}

struct SignalSocket {
    channel: Arc<Mutex<SignalChannel>>,
}

impl SignalSocket {
    fn new(channel: Arc<Mutex<SignalChannel>>) -> Self {
        return SignalSocket { channel };
    }
}

impl Actor for SignalSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        let resolved_channel: &mut SignalChannel = &mut self.channel.lock().unwrap();
        resolved_channel.join(context.address().recipient());
        println!("Signal Socket Opened")
    }

    fn stopped(&mut self, context: &mut Self::Context) {
        let resolved_channel: &mut SignalChannel = &mut self.channel.lock().unwrap();
        resolved_channel.exit(&context.address().recipient());
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
            Ok(ws::Message::Text(message)) => {
                if let Ok(signal_message) = SessionDescriptionMessage::parse_json(&message) {
                    let resolved_channel: &SignalChannel = &self.channel.lock().unwrap();
                    resolved_channel.broadcast(signal_message);
                }
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

impl Handler<Arc<SessionDescriptionMessage>> for SignalSocket {
    type Result = Result<(), MessageSendError>;

    fn handle(
        &mut self,
        message: Arc<SessionDescriptionMessage>,
        context: &mut Self::Context,
    ) -> Result<(), MessageSendError> {
        let resolved_message: &SessionDescriptionMessage = &message;
        context.text(serde_json::to_string(resolved_message)?);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
enum Signal {
    Offer(SessionDescriptionMessage),
    Answer(SessionDescriptionMessage),
    NewIceCandidate,
}

#[derive(Serialize, Deserialize, Clone)]
struct SessionDescriptionMessage {
    target: String,
    name: String,
}

impl SessionDescriptionMessage{
pub fn parse_json(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }
}

impl Message for SessionDescriptionMessage {
    type Result = Result<(), MessageSendError>;
}

struct SignalChannel {
    sockets: Vec<Recipient<Arc<SessionDescriptionMessage>>>,
}

impl Default for SignalChannel {
    fn default() -> Self {
        SignalChannel {
            sockets: Vec::new(),
        }
    }
}

impl SignalChannel {
    pub fn broadcast(&self, message: SessionDescriptionMessage) {
        let message_ptr = Arc::new(message);
        for socket in &self.sockets {
            socket.do_send(message_ptr.clone()).unwrap();
        }
    }

    pub fn join(&mut self, recipient: Recipient<Arc<SessionDescriptionMessage>>) {
        if self.check_not_exists(&recipient) {
            println!("joined");
            self.sockets.push(recipient);
        }
    }

    pub fn exit(&mut self, addr: &Recipient<Arc<SessionDescriptionMessage>>) {
        if let Some(position) = self
            .sockets
            .iter()
            .position(|existing_socket| existing_socket == addr)
        {
            self.sockets.remove(position);
        }
    }

    fn check_not_exists(&self, addr: &Recipient<Arc<SessionDescriptionMessage>>) -> bool {
        self.sockets
            .iter()
            .all(|existing_socket| existing_socket != addr)
    }
}

struct SignalServerState {
    channels: HashMap<u32, Arc<Mutex<SignalChannel>>>,
    count: u32,
}

impl SignalServerState {
    fn new() -> Self {
        return SignalServerState {
            channels: HashMap::new(),
            count: 0,
        };
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(Mutex::new(SignalServerState::new()));
    HttpServer::new(move || {
        App::new()
            .register_data(state.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource("/signal/{share_id}").to(index))
    })
    .bind("0.0.0.0:8080")?
    .start()
    .await
}
