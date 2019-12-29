use serde::de::{Error, MapAccess, Unexpected, Visitor};
use serde::{Deserialize, Deserializer};

use super::{IceCandidate, SessionDescriptionMessage, Signal};

impl<'de> Deserialize<'de> for Signal {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(SignalVisitor)
    }
}

pub struct SignalVisitor;
impl<'de> Visitor<'de> for SignalVisitor {
    type Value = Signal;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "couldn't parse Signal type")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        while let Some((key, value)) = map.next_entry()? as Option<(&'de str, &'de str)> {
            if key == "type" {
                return match value {
                    "offer" => Ok(Signal::Offer(SessionDescriptionVisitor.visit_map(map)?)),
                    "answer" => Ok(Signal::Answer(SessionDescriptionVisitor.visit_map(map)?)),
                    "new_ice_candidate" => {
                        Ok(Signal::NewIceCandidate(IceCandidateVisitor.visit_map(map)?))
                    }
                    "assign" => Ok(Signal::assign(
                        AssignedNameVisitor.visit_map(map)?.to_owned(),
                    )),
                    others => Err(M::Error::invalid_value(
                        Unexpected::Str(others),
                        &"offer, answer, new_ice_candidate, assign",
                    )),
                };
            }
        }

        return Err(M::Error::missing_field("type"));
    }
}

pub struct SessionDescriptionVisitor;
impl<'de> Visitor<'de> for SessionDescriptionVisitor {
    type Value = SessionDescriptionMessage;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "couldn't parse Signal type")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        let mut target = Err(M::Error::missing_field("target"));
        let mut name: Result<&'de str, M::Error> = Err(M::Error::missing_field("name"));
        let mut sdp = Err(M::Error::missing_field("sdp"));

        while let Some((key, value)) = map.next_entry()? {
            match key {
                "target" => target = Ok(value),
                "name" => name = Ok(value),
                "sdp" => sdp = Ok(value),
                _ => continue,
            }
        }

        Ok(SessionDescriptionMessage {
            name: name?.to_owned(),
            target: target?.to_owned(),
            sdp: sdp?.to_owned(),
        })
    }
}

pub struct IceCandidateVisitor;
impl<'de> Visitor<'de> for IceCandidateVisitor {
    type Value = IceCandidate;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "couldn't parse Signal type")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        let mut target: Result<&'de str, M::Error> = Err(M::Error::missing_field("target"));
        let mut candidate: Result<&'de str, M::Error> = Err(M::Error::missing_field("candidate"));

        while let Some((key, value)) = map.next_entry()? {
            match key {
                "target" => target = Ok(value),
                "candidate" => candidate = Ok(value),
                _ => continue,
            }
        }

        Ok(IceCandidate {
            target: target?.to_owned(),
            candidate: candidate?.to_owned(),
        })
    }
}

pub struct AssignedNameVisitor;
impl<'de> Visitor<'de> for AssignedNameVisitor {
    type Value = &'de str;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "couldn't parse Signal type")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        while let Some((key, value)) = map.next_entry()? as Option<(&'de str, &'de str)> {
            if key == "name" {
                return Ok(value);
            }
        }

        return Err(M::Error::missing_field("name"));
    }
}
