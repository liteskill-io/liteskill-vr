use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionType {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub item_type: String,
    pub path: Option<String>,
    pub architecture: Option<String>,
    pub description: String,
    pub analysis_status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemWithTags {
    #[serde(flatten)]
    pub item: Item,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub item_id: String,
    pub title: String,
    pub content: String,
    pub author: String,
    pub author_type: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteWithTags {
    #[serde(flatten)]
    pub note: Note,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemOfInterest {
    pub id: String,
    pub item_id: String,
    pub title: String,
    pub description: String,
    pub location: Option<String>,
    pub severity: Option<String>,
    pub author: String,
    pub author_type: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoiWithTags {
    #[serde(flatten)]
    pub ioi: ItemOfInterest,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub source_id: String,
    pub source_type: String,
    pub target_id: String,
    pub target_type: String,
    pub connection_type: String,
    pub description: String,
    pub author: String,
    pub author_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSummary {
    pub item: ItemWithTags,
    pub note_count: i64,
    pub ioi_count: i64,
    pub connection_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDetail {
    pub item: ItemWithTags,
    pub notes: Vec<NoteWithTags>,
    pub items_of_interest: Vec<IoiWithTags>,
    pub connections: Vec<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub parent_item_id: Option<String>,
    pub parent_item_name: Option<String>,
    pub title: String,
    pub snippet: String,
}
