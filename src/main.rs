use actix::prelude::{Actor, ActorContext, Addr, AsyncContext, Handler, StreamHandler};
use actix_web::{middleware, web, App, HttpRequest, HttpServer, Responder};
use actix_web_actors::ws;
use signal::Signal;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

mod signal;
mod signal_router;

type SignalServerStateData = web::Data<Mutex<SignalServerState>>;

async fn index(
    state: SignalServerStateData,
    request: HttpRequest,
    stream: web::Payload,
) -> impl Responder {
    let mut resolved_server_state = state.lock().unwrap();
    let user_name = Uuid::new_v4();
    ws::start(
        SignalSocket::new(
            user_name.to_hyphenated().to_string(),
            &mut resolved_server_state,
        ),
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
        context.text(Signal::assign(self.user_name.clone()).to_string());
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
pub enum MessageSendError {
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

impl Handler<Signal> for SignalSocket {
    type Result = Result<(), MessageSendError>;

    fn handle(
        &mut self,
        message: Signal,
        context: &mut Self::Context,
    ) -> Result<(), MessageSendError> {
        let resolved_message: &Signal = &message;
        context.text(serde_json::to_string(resolved_message)?);
        Ok(())
    }
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
