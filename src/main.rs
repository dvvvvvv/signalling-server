use std::sync::Arc;
use actix::prelude::{Actor, Addr};
use actix_web::{middleware, web, App, HttpRequest, HttpServer, Responder};
use actix_web_actors::ws;
use signal::Signal;
use uuid::Uuid;

use error::Error;
use signal_router::{ExitMessage, JoinMessage, SignalMessage, SignalRouter};
use signal_socket::SignalSocket;

mod error;
mod signal;
mod signal_router;
mod signal_socket;

type SignalServerStateData = web::Data<Arc<SignalServerState>>;

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
        SignalSocket::new(user_name.to_hyphenated(), &state.signal_router),
        &request,
        stream,
    )
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let signal_router = SignalRouter::default();
    let signal_router_addr = signal_router.start();
    let state = Arc::new(SignalServerState::new(signal_router_addr));
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
