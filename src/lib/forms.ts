// Form specs for every mutating MCP tool, in one place. Each builder returns a
// FormDesc the global ModalLayer renders and submits via mcp_call. This module is
// the single home of the tool-name literals, which keeps human/agent parity
// (scripts/check-parity.mjs) honest: a tool here == a UI affordance.
import type { FormDesc } from "./store";
import type {
  Claim,
  ConnectionType,
  Explanation,
  ItemSummary,
  ItemWithTags,
  OpenQuestion,
  State,
  Transition,
} from "./types";

const opts = (...vals: string[]): { value: string; label: string }[] =>
  vals.map((v) => ({ value: v, label: v }));

const SEVERITY = opts("critical", "high", "medium", "low", "info");
const CONFIDENCE = opts("low", "medium", "high");
const CLAIM_STATUS = opts("hypothesis", "supported", "refuted");
const QUESTION_STATUS = opts("open", "answered", "blocked", "superseded");
const PRIORITY = opts("low", "medium", "high");
const ANALYSIS_STATUS = opts("untouched", "in_progress", "reviewed");
const EXPL_STATUS = opts("draft", "reviewed");
const EXPL_TYPE = opts(
  "architecture",
  "protocol",
  "packet_format",
  "state_machine",
  "control_flow",
  "data_flow",
  "memory_layout",
  "object_lifecycle",
  "api_surface",
  "threat_model",
  "custom",
);
const EVIDENCE_TYPE = opts(
  "static_analysis",
  "dynamic_trace",
  "decompilation",
  "disassembly",
  "packet_capture",
  "test_case",
  "runtime_log",
  "human_observation",
  "agent_inference",
);
const STRENGTH = opts("weak", "moderate", "strong");

function newKey(prefix: string): string {
  const rand =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : String(Date.now());
  return `${prefix}.${rand}`;
}

// --- Items ---
export const itemCreateForm = (): FormDesc => ({
  title: "New item",
  tool: "item_create",
  fields: [
    { name: "name", label: "Name", type: "text", required: true },
    {
      name: "item_type",
      label: "Type",
      type: "text",
      required: true,
      placeholder: "elf, shared_object, config, script…",
    },
    { name: "path", label: "Path", type: "text" },
    { name: "architecture", label: "Architecture", type: "text" },
    { name: "description", label: "Description", type: "textarea" },
    { name: "tags", label: "Tags (comma-separated)", type: "tags" },
  ],
});

export const itemEditForm = (item: ItemWithTags): FormDesc => ({
  title: `Edit ${item.name}`,
  tool: "item_update",
  hidden: { id: item.id },
  initial: {
    name: item.name,
    description: item.description,
    analysis_status: item.analysis_status,
    tags: item.tags,
  },
  fields: [
    { name: "name", label: "Name", type: "text" },
    { name: "description", label: "Description", type: "textarea" },
    {
      name: "analysis_status",
      label: "Status",
      type: "select",
      options: ANALYSIS_STATUS,
    },
    { name: "tags", label: "Tags (comma-separated)", type: "tags" },
  ],
});

// --- Notes ---
const noteFields: FormDesc["fields"] = [
  { name: "title", label: "Title", type: "text", required: true },
  { name: "content", label: "Content", type: "textarea", required: true },
  { name: "tags", label: "Tags (comma-separated)", type: "tags" },
];
export const noteCreateForm = (itemId: string): FormDesc => ({
  title: "New note",
  tool: "note_create",
  hidden: { item_id: itemId },
  fields: noteFields,
});
export const noteEditForm = (note: {
  id: string;
  title: string;
  content: string;
  tags: string[];
}): FormDesc => ({
  title: "Edit note",
  tool: "note_update",
  hidden: { id: note.id },
  initial: { title: note.title, content: note.content, tags: note.tags },
  fields: noteFields,
});

// --- Items of interest (findings) ---
const ioiFields: FormDesc["fields"] = [
  { name: "title", label: "Title", type: "text", required: true },
  {
    name: "description",
    label: "Description",
    type: "textarea",
    required: true,
  },
  {
    name: "location",
    label: "Location",
    type: "text",
    placeholder: "0x… / symbol / line",
  },
  { name: "severity", label: "Severity", type: "select", options: SEVERITY },
  { name: "tags", label: "Tags (comma-separated)", type: "tags" },
];
export const ioiCreateForm = (itemId: string): FormDesc => ({
  title: "New finding",
  tool: "ioi_create",
  hidden: { item_id: itemId },
  fields: ioiFields,
});
export const ioiEditForm = (ioi: {
  id: string;
  title: string;
  description: string;
  location?: string;
  severity?: string;
  tags: string[];
}): FormDesc => ({
  title: "Edit finding",
  tool: "ioi_update",
  hidden: { id: ioi.id },
  initial: {
    title: ioi.title,
    description: ioi.description,
    location: ioi.location,
    severity: ioi.severity,
    tags: ioi.tags,
  },
  fields: ioiFields,
});

// --- Connections ---
export const connectionCreateForm = (
  sourceId: string,
  items: ItemSummary[],
  connectionTypes: ConnectionType[],
): FormDesc => ({
  title: "New connection",
  tool: "connection_create",
  hidden: { source_id: sourceId, source_type: "item", target_type: "item" },
  fields: [
    {
      name: "target_id",
      label: "Target item",
      type: "select",
      required: true,
      options: items
        .filter((i) => i.item.id !== sourceId)
        .map((i) => ({ value: i.item.id, label: i.item.name })),
    },
    {
      name: "connection_type",
      label: "Type",
      type: "select",
      required: true,
      options: connectionTypes.map((c) => ({ value: c.name, label: c.name })),
    },
    { name: "description", label: "Description", type: "textarea" },
  ],
});

// --- Vocabularies ---
export const tagCreateForm = (): FormDesc => ({
  title: "New tag",
  tool: "tag_create",
  fields: [
    { name: "name", label: "Name", type: "text", required: true },
    { name: "description", label: "Description", type: "text" },
    {
      name: "color",
      label: "Color (hex)",
      type: "text",
      placeholder: "#7AD3FF",
    },
  ],
});
export const connectionTypeCreateForm = (): FormDesc => ({
  title: "New connection type",
  tool: "connection_type_create",
  fields: [
    { name: "name", label: "Name", type: "text", required: true },
    { name: "description", label: "Description", type: "text" },
  ],
});
export const bulkDeleteForm = (): FormDesc => ({
  title: "Bulk delete",
  tool: "bulk_delete",
  submitLabel: "Delete matching",
  fields: [
    { name: "author", label: "Author", type: "text" },
    { name: "since", label: "Since (ISO 8601)", type: "text" },
    {
      name: "entity_type",
      label: "Entity type",
      type: "select",
      options: opts("note", "item_of_interest", "connection", "item"),
    },
  ],
});

// --- Explanations ---
const explFields: FormDesc["fields"] = [
  { name: "title", label: "Title", type: "text", required: true },
  {
    name: "explanation_type",
    label: "Type",
    type: "select",
    options: EXPL_TYPE,
  },
  { name: "summary", label: "Summary (short TL;DR)", type: "textarea" },
  {
    name: "confidence",
    label: "Confidence",
    type: "select",
    options: CONFIDENCE,
  },
  { name: "status", label: "Status", type: "select", options: EXPL_STATUS },
  { name: "tags", label: "Tags (comma-separated)", type: "tags" },
];
export const explanationCreateForm = (): FormDesc => ({
  title: "New explanation",
  tool: "explanation_upsert",
  hidden: { stable_key: newKey("explanation") },
  fields: explFields,
});
export const explanationEditForm = (e: Explanation): FormDesc => ({
  title: "Edit explanation",
  tool: "explanation_update",
  hidden: { id: e.id },
  initial: {
    title: e.title,
    explanation_type: e.explanation_type,
    summary: e.summary,
    confidence: e.confidence,
    status: e.status,
  },
  fields: explFields.filter((f) => f.name !== "tags"),
});

// --- Claims ---
const claimFields: FormDesc["fields"] = [
  { name: "text", label: "Claim", type: "textarea", required: true },
  { name: "status", label: "Status", type: "select", options: CLAIM_STATUS },
  {
    name: "confidence",
    label: "Confidence",
    type: "select",
    options: CONFIDENCE,
  },
];
export const claimCreateForm = (explanationId: string): FormDesc => ({
  title: "New claim",
  tool: "claim_create",
  hidden: { explanation_id: explanationId },
  fields: claimFields,
});
export const claimEditForm = (c: Claim): FormDesc => ({
  title: "Edit claim",
  tool: "claim_update",
  hidden: { id: c.id },
  initial: { text: c.text, status: c.status, confidence: c.confidence },
  fields: claimFields,
});

// --- Open questions ---
const questionFields: FormDesc["fields"] = [
  { name: "question", label: "Question", type: "textarea", required: true },
  { name: "priority", label: "Priority", type: "select", options: PRIORITY },
  { name: "status", label: "Status", type: "select", options: QUESTION_STATUS },
];
export const questionCreateForm = (explanationId: string): FormDesc => ({
  title: "New open question",
  tool: "open_question_create",
  hidden: { explanation_id: explanationId },
  fields: questionFields,
});
export const questionEditForm = (q: OpenQuestion): FormDesc => ({
  title: "Edit open question",
  tool: "open_question_update",
  hidden: { id: q.id },
  initial: { question: q.question, priority: q.priority, status: q.status },
  fields: questionFields,
});

// --- States ---
const stateFields: FormDesc["fields"] = [
  { name: "name", label: "Name", type: "text", required: true },
  { name: "description", label: "Description", type: "textarea" },
  { name: "is_initial", label: "Initial state", type: "checkbox" },
  { name: "is_terminal", label: "Terminal state", type: "checkbox" },
];
export const stateCreateForm = (explanationId: string): FormDesc => ({
  title: "New state",
  tool: "state_create",
  hidden: { explanation_id: explanationId },
  fields: stateFields,
});
export const stateEditForm = (s: State): FormDesc => ({
  title: "Edit state",
  tool: "state_update",
  hidden: { id: s.id },
  initial: {
    name: s.name,
    description: s.description,
    is_initial: s.is_initial,
    is_terminal: s.is_terminal,
  },
  fields: stateFields,
});

// --- Transitions ---
const transitionFields = (states: State[]): FormDesc["fields"] => {
  const stateOpts = states.map((s) => ({ value: s.stable_key, label: s.name }));
  return [
    {
      name: "from_state",
      label: "From",
      type: "select",
      required: true,
      options: stateOpts,
    },
    {
      name: "to_state",
      label: "To",
      type: "select",
      required: true,
      options: stateOpts,
    },
    { name: "event", label: "Event", type: "text" },
    { name: "guard", label: "Guard", type: "text" },
    { name: "action", label: "Action", type: "text" },
    { name: "description", label: "Description", type: "textarea" },
  ];
};
export const transitionCreateForm = (
  explanationId: string,
  states: State[],
): FormDesc => ({
  title: "New transition",
  tool: "transition_create",
  hidden: { explanation_id: explanationId },
  fields: transitionFields(states),
});
export const transitionEditForm = (
  t: Transition,
  states: State[],
): FormDesc => ({
  title: "Edit transition",
  tool: "transition_update",
  hidden: { id: t.id },
  initial: {
    from_state: t.from_state,
    to_state: t.to_state,
    event: t.event,
    guard: t.guard,
    action: t.action,
    description: t.description,
  },
  fields: transitionFields(states),
});

// --- Evidence ---
export const evidenceCreateForm = (
  targetType: "explanation" | "claim" | "finding",
  targetId: string,
): FormDesc => ({
  title: "Attach evidence",
  tool: "evidence_link",
  hidden: { target_type: targetType, target_id: targetId },
  fields: [
    {
      name: "external_locator",
      label: "Locator",
      type: "text",
      placeholder: "FUN_00401000+0x14",
    },
    {
      name: "external_kind",
      label: "Kind",
      type: "select",
      options: opts(
        "ghidra",
        "address",
        "pcap",
        "decompilation",
        "disassembly",
        "log",
        "test_case",
        "other",
      ),
    },
    {
      name: "evidence_type",
      label: "Evidence type",
      type: "select",
      options: EVIDENCE_TYPE,
    },
    { name: "strength", label: "Strength", type: "select", options: STRENGTH },
    { name: "excerpt", label: "Excerpt", type: "textarea" },
  ],
});
