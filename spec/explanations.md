# Explanations (knowledge layer)

> **Status: v1 shipped.** This page documents what exists today. Typed content
> models (state machines, packet formats, flows) are planned follow-ups — see
> [Roadmap](#roadmap).

A **finding** says "something dangerous exists here." An **explanation** says
"this is how the system works." Explanations make understanding a first-class,
evidence-backed citizen alongside items, notes, items of interest, and
connections — useful from the start of an engagement, before any bug is found.

## Model

An **Explanation** is a new first-class entity (its own `.lsvr` table). It is an
abstraction _over_ items, not an item itself.

```typescript
interface Explanation {
  id: string;
  stable_key: string; // client-provided, unique per project — upsert key
  title: string;
  explanation_type: string; // architecture | protocol | state_machine | control_flow |
  // data_flow | memory_layout | object_lifecycle | api_surface | threat_model | custom
  summary: string; // short TL;DR — substance goes in claims, not here
  status: "draft" | "reviewed";
  confidence: "low" | "medium" | "high";
  author: string;
  author_type: "human" | "agent";
  created_at: string;
  updated_at: string;
}
```

Substance lives in **claims** (each evidence-backed) and **open questions** (the
unknowns):

```typescript
interface Claim {
  id: string;
  explanation_id: string;
  stable_key: string; // unique per explanation
  text: string;
  claim_type: string; // behavior | invariant | constraint | assumption | hypothesis | …
  status: "hypothesis" | "supported" | "refuted";
  confidence: "low" | "medium" | "high";
  // + author, author_type, timestamps
}

interface OpenQuestion {
  id: string;
  explanation_id: string;
  stable_key: string; // unique per explanation
  question: string;
  priority: "low" | "medium" | "high";
  status: "open" | "answered" | "blocked" | "superseded";
  answer_claim_id?: string;
  // + author, author_type, timestamps
}
```

## Evidence

Evidence is the point of the layer. An **evidence link** attaches a source to a
target (an explanation, a claim, or a finding). The source is **either** an
existing entity **or** a free-text external locator — RE evidence is often a
Ghidra symbol or pcap packet that isn't its own DB row.

```typescript
interface EvidenceLink {
  id: string;
  target_type: "explanation" | "claim" | "finding";
  target_id: string;
  source_entity_type?: string; // item | item_of_interest | note | connection | explanation
  source_entity_id?: string;
  external_locator?: string; // e.g. "FUN_00401000+0x14", "pcap:42"
  external_kind?: string; // ghidra | address | pcap | decompilation | log | …
  evidence_type: string; // static_analysis | decompilation | packet_capture | …
  strength: "weak" | "moderate" | "strong";
  excerpt?: string;
  // + author, author_type, created_at
}
```

At least one of `source_entity_id` / `external_locator` is required.

## Links reuse connections

Coarse entity links are not a new system — they reuse the existing `connections`
table (which already powers the Connection Map):

- **scope**: `explanation --explains--> item` (set via `scope_item_ids` on upsert)
- **finding context**: `item_of_interest --affects--> explanation`

`explains` and `affects` are registered connection types, seeded by default.

## MCP tools

| Tool                                             | Purpose                                                                                                                                                                                  |
| ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `explanation_upsert`                             | Create/update an explanation **by `stable_key`** with nested `claims` + `open_questions` + `scope_item_ids`, all-or-nothing. Re-runs converge. Returns the detail + advisory `warnings`. |
| `explanation_get`                                | One explanation with claims, open questions, evidence, and scope.                                                                                                                        |
| `explanation_list`                               | List with child counts; filter by `explanation_type` / `status`.                                                                                                                         |
| `explanation_update` / `explanation_delete`      | Update envelope fields / delete an explanation (cascades children + scope/evidence links).                                                                                               |
| `claim_create` / `claim_update` / `claim_delete` | Granular CRUD for a claim (full CRUD parity).                                                                                                                                            |
| `open_question_create` / `_update` / `_delete`   | Granular CRUD for an open question.                                                                                                                                                      |
| `evidence_link` / `evidence_delete`              | Attach / remove evidence on an explanation, claim, or finding.                                                                                                                           |

Discovery reuses the existing read tools: `project_summary` includes an
`explanations` list and open `open_questions`; `changes_since` includes an
`explanations` group; `filter` accepts `entity_type` of `explanation` and
`open_question`.

### Idempotency

`stable_key` is the upsert key — re-running with the same keys updates rows in
place (no duplicates) and re-uses scope connections. This is what lets an agent
refine a model across sessions.

### The anti-wiki guardrail

`explanation_upsert` never blocks, but returns advisory `warnings`:

- a long `summary` with **no claims** → "prose dump" smell (keep the substance in claims)
- claims with **no linked evidence** → attach evidence or lower confidence

## UI

The desktop **Explanations** section lists explanations and shows a detail view:
summary, claims (with status + confidence badges and their evidence), open
questions (with priority), explanation-level evidence, and clickable scope
items. Humans get full CRUD here (create/edit explanations and each child,
delete with confirm) via the granular tools above — the UI auto-generates
`stable_key`s so humans never type them. Fed by the project snapshot; updates
live on `db-changed`.

## Roadmap

v1 ships the envelope + claims + questions + evidence. Planned next: typed
content models that lower into this layer — **state machine** first (states +
transitions + Mermaid render), then packet format and data flow — plus a human
review workflow once the UI gains write actions.
