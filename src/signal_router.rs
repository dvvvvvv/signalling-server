use super::signal::Signal;
use super::SignalSocket;
use actix::prelude::{Actor, Addr, Context, Handler, Message};
use futures::{FutureExt, TryFuture, TryFutureExt};
use std::collections::HashMap;

#[derive(Default)]
pub struct SignalRouter {
    sockets: HashMap<String, Addr<SignalSocket>>,
}

impl Actor for SignalRouter {
    type Context = Context<Self>;
}

impl SignalRouter {
    fn target(&self, target_name: &str) -> Option<&Addr<SignalSocket>> {
        self.sockets.get(target_name)
    }
}

trait RoutingTarget<T: Message, R> {
    fn route_message(&self, message: T) -> R;
}
impl RoutingTarget<Signal, <Signal as Message>::Result> for Option<&Addr<SignalSocket>> {
    fn route_message(&self, message: Signal) -> <Signal as Message>::Result {
        //TODO: error propagation, synchronous... almost everything is horrible. must be fixed.
        if let Some(existing_self) = self {
            existing_self.do_send(message);
            Ok(())
        } else {
            Ok(())
        }
    }
}

impl Handler<SignalMessage> for SignalRouter {
    type Result = <SignalMessage as Message>::Result;

    fn handle(&mut self, message: SignalMessage, context: &mut Self::Context) -> Self::Result {
        //TODO: this match is horrible to.
        match &message.0 {
            Signal::Answer(signal) | Signal::Offer(signal) => self
                .target(&signal.target)
                .route_message(message.0)
                .map_err(drop),
            Signal::NewIceCandidate(ice_candidate) => self
                .target(&ice_candidate.target)
                .route_message(message.0)
                .map_err(drop),
            _ => Ok(()), //do nothing
        }
    }
}

impl Handler<JoinMessage> for SignalRouter {
    type Result = <JoinMessage as Message>::Result;

    fn handle(&mut self, message: JoinMessage, context: &mut Self::Context) -> Self::Result {
        self.sockets
            .insert(message.user_name, message.signal_socket_addr);
        Ok(())
    }
}

impl Handler<ExitMessage> for SignalRouter {
    type Result = <JoinMessage as Message>::Result;

    fn handle(&mut self, message: ExitMessage, context: &mut Self::Context) -> Self::Result {
        self.sockets.remove(&message.0);
        Ok(())
    }
}

pub struct SignalMessage(Signal);

impl Message for SignalMessage {
    type Result = Result<(), ()>;
}

impl From<Signal> for SignalMessage {
    fn from(signal: Signal) -> Self {
        SignalMessage(signal)
    }
}

pub struct JoinMessage {
    user_name: String,
    signal_socket_addr: Addr<SignalSocket>,
}

impl JoinMessage {
    pub fn new(user_name: String, signal_socket_addr: Addr<SignalSocket>) -> Self {
        JoinMessage {
            user_name,
            signal_socket_addr,
        }
    }
}

impl Message for JoinMessage {
    type Result = Result<(), ()>;
}

pub struct ExitMessage(String);

impl Message for ExitMessage {
    type Result = Result<(), ()>;
}

impl From<String> for ExitMessage {
    fn from(name: String) -> Self {
        ExitMessage(name)
    }
}
