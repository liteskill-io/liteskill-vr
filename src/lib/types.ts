export interface Tag {
  id: string;
  name: string;
  description: string;
  color?: string;
  created_at: string;
}

export interface ConnectionType {
  id: string;
  name: string;
  description: string;
  created_at: string;
}

export interface Item {
  id: string;
  name: string;
  item_type: string;
  path?: string;
  architecture?: string;
  description: string;
  analysis_status: "untouched" | "in_progress" | "reviewed";
  created_at: string;
  updated_at: string;
}

export interface ItemWithTags extends Item {
  tags: string[];
}

export interface Note {
  id: string;
  item_id?: string;
  title: string;
  content: string;
  author: string;
  author_type: "human" | "agent";
  created_at: string;
  updated_at: string;
}

export interface NoteWithTags extends Note {
  tags: string[];
}

export interface ItemOfInterest {
  id: string;
  item_id: string;
  title: string;
  description: string;
  location?: string;
  severity?: "critical" | "high" | "medium" | "low" | "info";
  status: string;
  author: string;
  author_type: "human" | "agent";
  created_at: string;
  updated_at: string;
}

export interface IoiWithTags extends ItemOfInterest {
  tags: string[];
}

export interface Connection {
  id: string;
  source_id: string;
  source_type: "item" | "item_of_interest";
  target_id: string;
  target_type: "item" | "item_of_interest";
  connection_type: string;
  description: string;
  author: string;
  author_type: "human" | "agent";
  created_at: string;
}

export interface ItemSummary {
  item: ItemWithTags;
  note_count: number;
  ioi_count: number;
  connection_count: number;
}

export interface ItemDetail {
  item: ItemWithTags;
  notes: NoteWithTags[];
  items_of_interest: IoiWithTags[];
  connections: Connection[];
}

export interface Explanation {
  id: string;
  stable_key: string;
  title: string;
  explanation_type: string;
  summary: string;
  status: "draft" | "reviewed";
  confidence: "low" | "medium" | "high";
  author: string;
  author_type: "human" | "agent";
  created_at: string;
  updated_at: string;
}

export interface Claim {
  id: string;
  explanation_id: string;
  stable_key: string;
  text: string;
  claim_type: string;
  status: "hypothesis" | "supported" | "refuted";
  confidence: "low" | "medium" | "high";
  author: string;
  author_type: "human" | "agent";
  created_at: string;
  updated_at: string;
}

export interface OpenQuestion {
  id: string;
  explanation_id: string;
  stable_key: string;
  question: string;
  priority: "low" | "medium" | "high";
  status: "open" | "answered" | "blocked" | "superseded";
  answer_claim_id?: string;
  author: string;
  author_type: "human" | "agent";
  created_at: string;
  updated_at: string;
}

export interface EvidenceLink {
  id: string;
  target_type: "explanation" | "claim" | "finding";
  target_id: string;
  source_entity_type?: string;
  source_entity_id?: string;
  external_locator?: string;
  external_kind?: string;
  evidence_type: string;
  strength: "weak" | "moderate" | "strong";
  excerpt?: string;
  author: string;
  author_type: "human" | "agent";
  created_at: string;
}

export interface ExplanationSummary extends Explanation {
  tags: string[];
  scope_item_ids: string[];
  claim_count: number;
  open_question_count: number;
  evidence_count: number;
}

export interface State {
  id: string;
  explanation_id: string;
  stable_key: string;
  name: string;
  description: string;
  is_initial: boolean;
  is_terminal: boolean;
  author: string;
  author_type: "human" | "agent";
  created_at: string;
  updated_at: string;
}

export interface Transition {
  id: string;
  explanation_id: string;
  stable_key: string;
  from_state: string;
  to_state: string;
  event: string;
  guard?: string;
  action?: string;
  description: string;
  author: string;
  author_type: "human" | "agent";
  created_at: string;
  updated_at: string;
}

export interface ExplanationDetail extends Explanation {
  tags: string[];
  scope_item_ids: string[];
  claims: Claim[];
  open_questions: OpenQuestion[];
  evidence: EvidenceLink[];
  states: State[];
  transitions: Transition[];
  diagram_text?: string;
}

export interface ProjectSnapshot {
  items: ItemSummary[];
  details: ItemDetail[];
  tags: Tag[];
  connection_types: ConnectionType[];
  explanations: ExplanationSummary[];
  explanation_details: ExplanationDetail[];
  mcp_port: number;
}
