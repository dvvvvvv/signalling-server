use super::Signal;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

impl Serialize for Signal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Signal::Offer(sdp_signal) => {
                let mut map = serializer.serialize_map(Some(4))?;
                map.serialize_entry("type", "offer")?;
                map.serialize_entry("name", &sdp_signal.name)?;
                map.serialize_entry("target", &sdp_signal.target)?;
                map.serialize_entry("sdp", &sdp_signal.sdp)?;
                map.end()
            }
            Signal::Answer(sdp_signal) => {
                let mut map = serializer.serialize_map(Some(4))?;
                map.serialize_entry("type", "answer")?;
                map.serialize_entry("name", &sdp_signal.name)?;
                map.serialize_entry("target", &sdp_signal.target)?;
                map.serialize_entry("sdp", &sdp_signal.sdp)?;
                map.end()
            }
            Signal::NewIceCandidate(ice_candidate) => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("type", "new_ice_candidate")?;
                map.serialize_entry("target", &ice_candidate.target)?;
                map.serialize_entry("candidate", &ice_candidate.candidate)?;
                map.end()
            }
            Signal::Assign(user_name) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("type", "assign")?;
                map.serialize_entry("name", &user_name)?;
                map.end()
            }
        }
    }
}
