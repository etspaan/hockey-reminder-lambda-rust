use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum EventType {
    #[serde(rename = "g")]
    Game,
    #[serde(rename = "L")]
    LockerRoom,
    #[serde(other)]
    Unknown,
}