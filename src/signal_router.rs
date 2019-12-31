use super::signal::Signal;
use actix::fut::wrap_future;
use actix::prelude::{Actor, Context, Handler, Message, Recipient, ResponseActFuture};
use futures::TryFutureExt;
use std::collections::HashMap;
use std::future::Future;

use super::Error;

#[derive(Default)]
pub struct SignalRouter {
    sockets: HashMap<String, Recipient<Signal>>,
}

impl Actor for SignalRouter {
    type Context = Context<Self>;
}

impl SignalRouter {
    fn target(&self, target_name: &str) -> Option<&Recipient<Signal>> {
        self.sockets.get(target_name)
    }

    fn wrap_future<F>(future: F) -> ResponseActFuture<Self, Result<(), Error>>
    where
        F: Future<Output = Result<(), Error>> + 'static,
    {
        Box::new(wrap_future(future))
    }
}

impl Handler<SignalMessage> for SignalRouter {
    type Result = ResponseActFuture<Self, Result<(), Error>>;

    fn handle(&mut self, message: SignalMessage, _: &mut Self::Context) -> Self::Result {
        match &message.0 {
            Signal::Answer(signal) | Signal::Offer(signal) => {
                if let Some(target_socket) = self.target(&signal.target) {
                    let message_transfer_future = target_socket
                        .send(message.0)
                        .unwrap_or_else(|mailbox_err| Err(into_target_related_error(mailbox_err)));
                    Self::wrap_future(message_transfer_future)
                } else {
                    Self::wrap_future(futures::future::err(Error::TargetNotFound(
                        signal.target.clone(),
                    )))
                }
            }
            Signal::NewIceCandidate(ice_candidate) => {
                if let Some(target_socket) = self.target(&ice_candidate.target) {
                    let message_transfer_future = target_socket
                        .send(message.0)
                        .unwrap_or_else(|mailbox_err| Err(into_target_related_error(mailbox_err)));
                    Self::wrap_future(message_transfer_future)
                } else {
                    Self::wrap_future(futures::future::err(Error::TargetNotFound(
                        ice_candidate.target.clone(),
                    )))
                }
            }
            _ => Self::wrap_future(futures::future::ok(())), //do nothing
        }
    }
}

fn into_target_related_error(mailbox_error: actix::MailboxError) -> Error {
    match mailbox_error {
        actix::MailboxError::Closed => Error::ConnectionClosed,
        actix::MailboxError::Timeout => Error::ConnectionTimeout,
    }
}

impl Handler<JoinMessage> for SignalRouter {
    type Result = <JoinMessage as Message>::Result;

    fn handle(&mut self, message: JoinMessage, _: &mut Self::Context) -> Self::Result {
        self.sockets
            .insert(message.user_name, message.signal_recipient);
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
    type Result = Result<(), Error>;
}

impl From<Signal> for SignalMessage {
    fn from(signal: Signal) -> Self {
        SignalMessage(signal)
    }
}

pub struct JoinMessage {
    user_name: String,
    signal_recipient: Recipient<Signal>,
}

impl JoinMessage {
    pub fn new(user_name: String, signal_recipient: Recipient<Signal>) -> Self {
        JoinMessage {
            user_name,
            signal_recipient,
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

#[cfg(test)]
#[derive(Default)]
struct MockSignalHandler {
    last_received_message: std::sync::Arc<std::sync::Mutex<Option<Signal>>>,
}

#[cfg(test)]
impl Actor for MockSignalHandler {
    type Context = Context<Self>;
}

#[cfg(test)]
impl Handler<Signal> for MockSignalHandler {
    type Result = <Signal as Message>::Result;

    fn handle(&mut self, message: Signal, _: &mut Self::Context) -> Self::Result {
        self.last_received_message.lock().unwrap().replace(message);
        Ok(())
    }
}

#[actix_rt::test]
async fn test_routing() -> std::io::Result<()> {
    let router_addr = SignalRouter::default().start();
    let caller_actor = MockSignalHandler::default();
    let caller_actor_addr = caller_actor.start();
    let callee_actor = MockSignalHandler::default();
    let message_placeholder = callee_actor.last_received_message.clone();
    let callee_actor_addr = callee_actor.start();

    let caller_join_fut = router_addr.send(JoinMessage::new(
        "caller".to_owned(),
        caller_actor_addr.recipient(),
    ));
    let callee_join_fut = router_addr.send(JoinMessage::new(
        "callee".to_owned(),
        callee_actor_addr.recipient(),
    ));

    assert!(caller_join_fut.await.is_ok());
    assert!(callee_join_fut.await.is_ok());

    let signal_text = r#"{"type":"offer","name":"caller","target":"callee","sdp":"dummy sdp"}"#;
    let offer_signal: Signal = serde_json::from_str(signal_text).unwrap();

    let signal_routing_fut = router_addr.send(SignalMessage::from(offer_signal.clone()));
    assert!(signal_routing_fut.await.is_ok());

    let resolved_signal_ref: &mut Option<Signal> = &mut message_placeholder.lock().unwrap();
    assert!(resolved_signal_ref.is_some());
    assert_eq!(&offer_signal, resolved_signal_ref.as_ref().unwrap());

    Ok(())
}
