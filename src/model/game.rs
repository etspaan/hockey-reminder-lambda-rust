use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct GameInfo {
    pub dt: DateTime<Utc>,
    pub h_id: Option<i64>,
    pub v_id: Option<i64>,
    pub res_id: Option<i64>,
    pub home_locker_room: Option<String>,
    pub away_locker_room: Option<String>,
}