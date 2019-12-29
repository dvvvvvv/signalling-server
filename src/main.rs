use actix::prelude::{Actor, ActorContext, Addr, AsyncContext, Handler, StreamHandler};
use actix_web::{middleware, web, App, HttpRequest, HttpServer, Responder};
use actix_web_actors::ws;
use futures::executor::block_on;
use signal::Signal;
use uuid::Uuid;

use signal_router::{ExitMessage, JoinMessage, SignalRouter, SignalMessage};

mod signal;
mod signal_router;

type SignalServerStateData = web::Data<SignalServerState>;

async fn signal(
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
        SignalSocket {
            user_name,
            signal_router: signal_router.clone(),
        }
    }
}

impl SignalSocket {
    async fn handle_signal_message(&self, signal_message: Signal, context: &mut ws::WebsocketContext<Self>) {
        let signal_routing_result = self.signal_router.send(SignalMessage::from(signal_message))
            .await
            .unwrap_or_else(|mailbox_error| Err(into_service_releated_error(mailbox_error)));
        match signal_routing_result {
            Ok(_) => {},//do nothing
            Err(err) => context.text(&serde_json::to_string(&ErrorMessage::from(err)).unwrap()),
        }
    }
}

fn into_service_releated_error(mailbox_error: actix::MailboxError) -> MessageSendError {
    match mailbox_error {
        actix::MailboxError::Closed => MessageSendError::ServiceUnavailable,
        actix::MailboxError::Timeout => MessageSendError::ServiceTimeout,
    }
}

impl Actor for SignalSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        let joining_router_fut = self
            .signal_router
            .send(JoinMessage::new(self.user_name.clone(), context.address()));

        if block_on(joining_router_fut).is_ok() {
            context.text(Signal::assign(self.user_name.clone()).to_string());
            println!("Signal Socket Opened")
        } else {
            context.stop();
        }
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        let exiting_router_fut = self.signal_router
            .send(ExitMessage::from(self.user_name.clone()));

        if block_on(exiting_router_fut).is_ok() {
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
                block_on(self.handle_signal_message(signal, context))
            } else {
                context.text("couldn't parse your message")
            },
            Ok(_) => {
                println!("some message received.");
            }
            Err(error) => eprintln!("error occurred during receive message: {}", error),
        }
    }
}

#[derive(serde::Serialize)]
struct ErrorMessage {
    r#type: &'static str,
    message: String,
}

impl From<MessageSendError> for ErrorMessage {
    fn from(message_send_error:MessageSendError) -> Self {
        match message_send_error {
            MessageSendError::ParseError(parse_error) => ErrorMessage {
                r#type: "parse error",
                message: format!("{}", parse_error),
            },
            MessageSendError::ConnectionClosed => ErrorMessage {
                r#type: "connection closed",
                message: "target user's connection is closed".to_owned(),
            },
            MessageSendError::ConnectionTimeout => ErrorMessage {
                r#type: "timeout",
                message: "timeout occurres during send message to target user".to_owned(),
            },
            MessageSendError::TargetNotFound(target_user_name) => ErrorMessage {
                r#type: "target user not found",
                message: format!("user {} is not in connection", target_user_name)
            },
            MessageSendError::ServiceUnavailable => ErrorMessage {
                r#type: "service unavailable",
                message: "service is unavailable, please contact to service provider".to_owned(),
            },
            MessageSendError::ServiceTimeout => ErrorMessage {
                r#type: "service timeout",
                message: "service is busy. try after".to_owned(),
            }
        }
    }
}

#[derive(Debug)]
pub enum MessageSendError {
    ParseError(serde_json::Error),
    ConnectionClosed,
    ConnectionTimeout,
    TargetNotFound(String),
    ServiceUnavailable,
    ServiceTimeout,
}

impl From<serde_json::Error> for MessageSendError {
    fn from(err: serde_json::Error) -> MessageSendError {
        Self::ParseError(err)
    }
}

impl std::fmt::Display for MessageSendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(err) => write!(formatter, "ParseError({})", err),
            Self::ConnectionClosed => write!(formatter, "ConnectionClosed"),
            Self::ConnectionTimeout => write!(formatter, "ConnectionTimeout"),
            Self::TargetNotFound(target_user_name) => write!(formatter, "TargetNotFound(target_user_name: {})", target_user_name),
            Self::ServiceUnavailable => write!(formatter, "ServiceUnavailable"),
            Self::ServiceTimeout => write!(formatter, "ServiceTemporaryUnavailable")
        }
    }
}

impl std::error::Error for MessageSendError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            Self::ParseError(err) => Some(err),
            _ => None,
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
        context.text(serde_json::to_string(&message)?);
        Ok(())
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
            .data(state.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource("/signal").to(signal))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
