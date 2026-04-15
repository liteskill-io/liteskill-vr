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
