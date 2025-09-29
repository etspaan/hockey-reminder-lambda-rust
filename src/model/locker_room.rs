use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LockerRoomAttributes {
    pub name: Option<String>,
    pub description: Option<String>,
}