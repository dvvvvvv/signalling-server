use super::signal::Signal;
use super::SignalSocket;
use actix::prelude::{Actor, Addr, Context, Handler, Message, ResponseActFuture};
use actix::fut::wrap_future;
use std::future::Future;
use futures::TryFutureExt;
use std::collections::HashMap;

use super::MessageSendError;

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

    fn wrap_future <F> (future: F) -> ResponseActFuture<Self, Result<(), MessageSendError>> 
    where F: Future<Output=Result<(), MessageSendError>> + 'static {
        Box::new(wrap_future(future))
    }
}

impl Handler<SignalMessage> for SignalRouter {
    type Result = ResponseActFuture<Self, Result<(), MessageSendError>>;

    fn handle(&mut self, message: SignalMessage, _: &mut Self::Context) -> Self::Result {
        match &message.0 {
            Signal::Answer(signal) | Signal::Offer(signal) => {
                if let Some(target_socket) = self.target(&signal.target) {
                    let message_transfer_future = target_socket.send(message.0).unwrap_or_else(|mailbox_err| Err(into_target_related_error(mailbox_err)));
                    Self::wrap_future(message_transfer_future)
                } else {
                    Self::wrap_future(futures::future::err(MessageSendError::TargetNotFound(signal.target.clone())))
                }
            },
            Signal::NewIceCandidate(ice_candidate) =>  {
                if let Some(target_socket) = self.target(&ice_candidate.target) {
                    let message_transfer_future = target_socket.send(message.0).unwrap_or_else(|mailbox_err| Err(into_target_related_error(mailbox_err)));
                    Self::wrap_future(message_transfer_future)
                } else {
                    Self::wrap_future(futures::future::err(MessageSendError::TargetNotFound(ice_candidate.target.clone())))
                }
            }
            _ => Self::wrap_future(futures::future::ok(())), //do nothing
        }
    }
}

fn into_target_related_error(mailbox_error: actix::MailboxError) -> MessageSendError {
    match mailbox_error {
        actix::MailboxError::Closed => MessageSendError::ConnectionClosed,
        actix::MailboxError::Timeout => MessageSendError::ConnectionTimeout,
    }
}

impl Handler<JoinMessage> for SignalRouter {
    type Result = <JoinMessage as Message>::Result;

    fn handle(&mut self, message: JoinMessage, _: &mut Self::Context) -> Self::Result {
        self.sockets
            .insert(message.user_name, message.signal_socket_addr);
        Ok(())
    }
}

impl Handler<ExitMessage> for SignalRouter {
    type Result = <JoinMessage as Message>::Result;

    fn handle(&mut self, message: ExitMessage, _: &mut Self::Context) -> Self::Result {
        self.sockets.remove(&message.0);
        Ok(())
    }
}

pub struct SignalMessage(Signal);

impl Message for SignalMessage {
    type Result = Result<(), MessageSendError>;
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
