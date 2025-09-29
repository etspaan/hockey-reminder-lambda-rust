use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct GameCore {
    pub dt: DateTime<Utc>,
    pub h_id: Option<i64>,
    pub v_id: Option<i64>,
    pub res_id: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct GameInfo {
    pub dt: DateTime<Utc>,
    pub h_id: Option<i64>,
    pub v_id: Option<i64>,
    pub res_id: Option<i64>,
    // Locker room resource IDs (resolved to names at formatting time to avoid cloning)
    pub home_locker_res_id: Option<i64>,
    pub away_locker_res_id: Option<i64>,
}