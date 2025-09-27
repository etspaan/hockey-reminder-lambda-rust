use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamDocument {
    pub data: Team,
    #[serde(default)]
    pub included: Vec<Included>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub attributes: TeamAttributes,
    #[serde(default)]
    pub relationships: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamAttributes {
    pub name: String,
    pub season_id: Option<i64>,
    pub league_id: Option<i64>,
    pub start_date: Option<String>,
    pub has_upcoming_events: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamBasicAttributes {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Included {
    #[serde(rename = "events")]
    Event {
        id: String,
        attributes: crate::model::event::EventAttributes,
        #[serde(default)]
        relationships: Option<serde_json::Value>,
    },
    #[serde(rename = "teams")]
    TeamIncluded {
        id: String,
        attributes: TeamBasicAttributes,
        #[serde(default)]
        relationships: Option<serde_json::Value>,
    },
    #[serde(rename = "resources")]
    Resource {
        id: String,
        attributes: crate::model::resource::ResourceAttributes,
        #[serde(default)]
        relationships: Option<serde_json::Value>,
    },
    #[serde(other)]
    Other,
}
