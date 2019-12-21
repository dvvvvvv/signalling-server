use super::MessageSendError;
use actix::Message;
use serde_json;

mod deserialize;
mod serialize;

#[derive(Clone)]
pub enum Signal {
    Offer(SessionDescriptionMessage),
    Answer(SessionDescriptionMessage),
    NewIceCandidate(IceCandidate),
    Assign(String),
}

#[derive(Clone)]
pub struct SessionDescriptionMessage {
    target: String,
    name: String,
    sdp: String,
}

#[derive(Clone)]
pub struct IceCandidate {
    target: String,
    candidate: String,
}

impl Signal {
    pub fn assign(user_name: String) -> Signal {
        Signal::Assign(user_name)
    }

    pub fn parse_json(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }
}

impl Message for Signal {
    type Result = Result<(), MessageSendError>;
}

impl ToString for Signal {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
