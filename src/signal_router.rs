use std::collections::HashMap;
use actix::prelude::{Actor, Context, Handler, Message, Addr};
use super::signal::Signal;
use super::SignalSocket;

#[derive(Default)]
pub struct SignalRouter {
    sockets: HashMap<String, Addr<SignalSocket>>,
}

impl Actor for SignalRouter {
    type Context = Context<Self>;
}

impl SignalRouter {
    fn target(&self, target_name:&str) -> Option<&Addr<SignalSocket>> {
        self.sockets.get(target_name)
    }
}

trait RoutingTarget<T:Message, R> {
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
            Signal::Answer(signal) | Signal::Offer(signal) => self.target(&signal.target).route_message(message.0).map_err(drop),
            Signal::NewIceCandidate(ice_candidate) => self.target(&ice_candidate.target).route_message(message.0).map_err(drop),
            _ => Ok(())//do nothing
        }
    }
}

struct SignalMessage(Signal);

impl Message for SignalMessage {
    type Result = Result<(),()>;
}

impl From<Signal> for SignalMessage {
    fn from(signal: Signal) -> Self {
        SignalMessage(signal)
    }
}

