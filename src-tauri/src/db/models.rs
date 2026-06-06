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
    pub item_id: Option<String>,
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
    pub status: String,
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

// --- Explanation layer ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    pub id: String,
    pub stable_key: String,
    pub title: String,
    pub explanation_type: String,
    pub summary: String,
    pub status: String,
    pub confidence: String,
    pub author: String,
    pub author_type: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub explanation_id: String,
    pub stable_key: String,
    pub text: String,
    pub claim_type: String,
    pub status: String,
    pub confidence: String,
    pub author: String,
    pub author_type: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenQuestion {
    pub id: String,
    pub explanation_id: String,
    pub stable_key: String,
    pub question: String,
    pub priority: String,
    pub status: String,
    pub answer_claim_id: Option<String>,
    pub author: String,
    pub author_type: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceLink {
    pub id: String,
    pub target_type: String,
    pub target_id: String,
    pub source_entity_type: Option<String>,
    pub source_entity_id: Option<String>,
    pub external_locator: Option<String>,
    pub external_kind: Option<String>,
    pub evidence_type: String,
    pub strength: String,
    pub excerpt: Option<String>,
    pub author: String,
    pub author_type: String,
    pub created_at: String,
}

/// Listing row for an explanation, with child counts (no children inlined).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationSummary {
    #[serde(flatten)]
    pub explanation: Explanation,
    pub tags: Vec<String>,
    pub scope_item_ids: Vec<String>,
    pub claim_count: i64,
    pub open_question_count: i64,
    pub evidence_count: i64,
}

/// A state in a `state_machine` explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub id: String,
    pub explanation_id: String,
    pub stable_key: String,
    pub name: String,
    pub description: String,
    pub is_initial: bool,
    pub is_terminal: bool,
    pub author: String,
    pub author_type: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A transition between two states (referenced by their `stable_key`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub id: String,
    pub explanation_id: String,
    pub stable_key: String,
    pub from_state: String,
    pub to_state: String,
    pub event: String,
    pub guard: Option<String>,
    pub action: Option<String>,
    pub description: String,
    pub author: String,
    pub author_type: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Full explanation with its claims, open questions, evidence, scope, and (for
/// state machines) states + transitions. `diagram_text` is a generated-on-the-fly
/// text rendering for agents — never stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationDetail {
    #[serde(flatten)]
    pub explanation: Explanation,
    pub tags: Vec<String>,
    pub scope_item_ids: Vec<String>,
    pub claims: Vec<Claim>,
    pub open_questions: Vec<OpenQuestion>,
    pub evidence: Vec<EvidenceLink>,
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagram_text: Option<String>,
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
