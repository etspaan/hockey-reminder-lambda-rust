use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LockerRoom {
    pub id: String,
    pub description: String,
}