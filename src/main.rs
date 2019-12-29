use actix::prelude::{Actor, ActorContext, Addr, AsyncContext, Handler, StreamHandler, ResponseActFuture};
use actix::fut::wrap_future;
use actix_web::{middleware, web, App, HttpRequest, HttpServer, Responder};
use actix_web_actors::ws;
use std::future::Future;
use futures::executor::block_on;
use signal::Signal;
use uuid::Uuid;

use signal_router::{ExitMessage, JoinMessage, SignalRouter, SignalMessage};

mod signal;
mod signal_router;

type SignalServerStateData = web::Data<SignalServerState>;

async fn index(
    state: SignalServerStateData,
    request: HttpRequest,
    stream: web::Payload,
) -> impl Responder {
    let user_name = Uuid::new_v4();
    ws::start(
        SignalSocket::new(
            user_name.to_hyphenated().to_string(),
            &state.signal_router
        ),
        &request,
        stream,
    )
}

pub struct SignalSocket {
    user_name: String,
    signal_router: Addr<SignalRouter>,
}

impl SignalSocket {
    fn new(user_name: String, signal_router: &Addr<SignalRouter>) -> Self {
        let new_signal_socket = SignalSocket {
            user_name,
            signal_router: signal_router.clone(),
        };

        return new_signal_socket;
    }
}

impl SignalSocket {
    fn handle_signal_message(&self, signal_message: Signal) {
        self.signal_router.send(SignalMessage::from(signal_message))
    }
}

impl Actor for SignalSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        let joining_router_fut = self
            .signal_router
            .send(JoinMessage::new(self.user_name.clone(), context.address()));

        if let Ok(_) = block_on(joining_router_fut) {
            context.text(Signal::assign(self.user_name.clone()).to_string());
            println!("Signal Socket Opened")
        } else {
            context.stop();
        }
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        let exiting_router_fut = self.signal_router
            .send(ExitMessage::from(self.user_name.clone()));

        if let Ok(_) = block_on(exiting_router_fut) {
            println!("Signal Socket Closed")
        } else {
            eprintln!("couldn't exit from router. user name: {}", self.user_name)
        }
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
            Ok(ws::Message::Text(text_message)) => if let Ok(signal) = serde_json::from_str(&text_message) {
                self.handle_signal_message(signal)
            } else {
                context.text("couldn't parse your message")
            },
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
    type Result = Result::<(),MessageSendError>;

    fn handle(
        &mut self,
        message: Signal,
        context: &mut Self::Context,
    ) -> Self::Result {
        Ok(context.text(serde_json::to_string(&message)?))
    }
}

struct SignalServerState {
    signal_router: Addr<SignalRouter>,
}

impl SignalServerState {
    fn new(signal_router: Addr<SignalRouter>) -> Self {
        SignalServerState { signal_router }
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let signal_router = SignalRouter::default();
    let signal_router_addr = signal_router.start();
    let state = web::Data::new(SignalServerState::new(signal_router_addr));
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
