use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EventAttributes {
    pub event_type_id: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub start_date: Option<String>,
    pub event_start_time: Option<String>,
    pub start_gmt: Option<String>,
    pub hteam_id: Option<i64>,
    pub vteam_id: Option<i64>,
    pub resource_id: Option<i64>,
    pub sub_type: Option<String>,
    // Additional fields used for locker room events to associate with a game
    pub parent_event_id: Option<i64>,
    pub locker_room_type: Option<String>,
}
