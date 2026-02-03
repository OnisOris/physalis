//! Client <-> server message protocol.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMsg {
    Hello {
        client_version: String,
    },
    AddBox {
        w: f32,
        h: f32,
        d: f32,
    },
    AddCylinder {
        r: f32,
        h: f32,
    },
    RequestHeavy {
        kind: String,
        payload: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    HelloAck,
    Log { text: String },
    JobAccepted { job_id: u64 },
    JobResult { job_id: u64, payload: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_msg_roundtrip() {
        let msg = ClientMsg::AddBox {
            w: 1.0,
            h: 2.0,
            d: 3.0,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ClientMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, back);
    }

    #[test]
    fn server_msg_roundtrip() {
        let msg = ServerMsg::JobResult {
            job_id: 42,
            payload: "ok".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: ServerMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, back);
    }
}
