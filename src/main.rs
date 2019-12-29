use actix::prelude::{Actor, Addr};
use actix_web::{middleware, web, App, HttpRequest, HttpServer, Responder};
use actix_web_actors::ws;
use signal::Signal;
use uuid::Uuid;

use signal_router::{ExitMessage, JoinMessage, SignalRouter, SignalMessage};
use signal_socket::SignalSocket;

mod signal;
mod signal_socket;
mod signal_router;

type SignalServerStateData = web::Data<SignalServerState>;

#[derive(serde::Serialize)]
struct ErrorMessage {
    r#type: &'static str,
    message: String,
}

impl From<Error> for ErrorMessage {
    fn from(message_send_error:Error) -> Self {
        match message_send_error {
            Error::ParseError(parse_error) => ErrorMessage {
                r#type: "parse error",
                message: format!("{}", parse_error),
            },
            Error::ConnectionClosed => ErrorMessage {
                r#type: "connection closed",
                message: "target user's connection is closed".to_owned(),
            },
            Error::ConnectionTimeout => ErrorMessage {
                r#type: "timeout",
                message: "timeout occurres during send message to target user".to_owned(),
            },
            Error::TargetNotFound(target_user_name) => ErrorMessage {
                r#type: "target user not found",
                message: format!("user {} is not in connection", target_user_name)
            },
            Error::ServiceUnavailable => ErrorMessage {
                r#type: "service unavailable",
                message: "service is unavailable, please contact to service provider".to_owned(),
            },
            Error::ServiceTimeout => ErrorMessage {
                r#type: "service timeout",
                message: "service is busy. try after".to_owned(),
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    ParseError(serde_json::Error),
    ConnectionClosed,
    ConnectionTimeout,
    TargetNotFound(String),
    ServiceUnavailable,
    ServiceTimeout,
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Self::ParseError(err)
    }
}

impl std::fmt::Display for Error {
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

impl std::error::Error for Error {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            Self::ParseError(err) => Some(err),
            _ => None,
        }
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
