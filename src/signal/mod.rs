use super::Error;
use actix::Message;
use serde_json;

mod deserialize;
mod serialize;

#[derive(Clone, Debug, PartialEq)]
pub enum Signal {
    Offer(SessionDescriptionMessage),
    Answer(SessionDescriptionMessage),
    NewIceCandidate(IceCandidate),
    Assign(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionDescriptionMessage {
    pub target: String,
    pub name: String,
    sdp: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IceCandidate {
    pub target: String,
    candidate: String,
}

impl Signal {
    pub fn assign(user_name: String) -> Signal {
        Signal::Assign(user_name)
    }
}

impl Message for Signal {
    type Result = Result<(), Error>;
}

impl ToString for Signal {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
