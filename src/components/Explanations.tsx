import {
  claimCreateForm,
  claimEditForm,
  evidenceCreateForm,
  explanationCreateForm,
  explanationEditForm,
  fieldCreateForm,
  fieldEditForm,
  questionCreateForm,
  questionEditForm,
  stateCreateForm,
  stateEditForm,
  transitionCreateForm,
  transitionEditForm,
} from "@/lib/forms";
import { useStore } from "@/lib/store";

import type {
  Claim,
  EvidenceLink,
  ExplanationDetail,
  Field,
  OpenQuestion,
  State,
  Transition,
} from "@/lib/types";

// Custom on-the-fly state-machine renderer (not Mermaid). States are laid out in
// a column; transitions are curved arrows in the right gutter, labelled
// event [guard] / action. Generated from the editable rows in the snapshot.
function StateMachineDiagram({
  states,
  transitions,
}: {
  states: State[];
  transitions: Transition[];
}): React.JSX.Element | null {
  if (states.length === 0) return null;
  const boxW = 190;
  const boxH = 40;
  const gapY = 64;
  const pad = 16;
  const gutter = 190;
  const idx = new Map(states.map((s, i) => [s.stable_key, i] as const));
  const midY = (i: number): number => pad + i * gapY + boxH / 2;
  const right = pad + boxW;
  const height = pad * 2 + (states.length - 1) * gapY + boxH;
  const width = right + gutter;

  const label = (t: Transition): string =>
    t.event +
    (t.guard ? ` [${t.guard}]` : "") +
    (t.action ? ` / ${t.action}` : "");

  return (
    <svg
      viewBox={`0 0 ${String(width)} ${String(height)}`}
      width="100%"
      style={{ maxWidth: width }}
      role="img"
      aria-label="State machine diagram"
      className="rounded-sm border border-border bg-surface"
      fontFamily="ui-monospace, monospace"
    >
      <defs>
        <marker
          id="sm-arrow"
          markerWidth="8"
          markerHeight="8"
          refX="7"
          refY="3"
          orient="auto"
        >
          <path d="M0 0 L7 3 L0 6 Z" fill="#8a83bf" />
        </marker>
      </defs>
      {transitions.map((t, n) => {
        const fi = idx.get(t.from_state);
        const ti = idx.get(t.to_state);
        if (fi === undefined || ti === undefined) return null;
        const y1 = midY(fi);
        const y2 = midY(ti);
        const lane = right + 18 + (n % 4) * 34;
        if (fi === ti) {
          const d = `M ${String(right)} ${String(y1 - 8)} C ${String(lane + 16)} ${String(y1 - 26)}, ${String(lane + 16)} ${String(y1 + 26)}, ${String(right)} ${String(y1 + 8)}`;
          return (
            <g key={t.id}>
              <path
                d={d}
                fill="none"
                stroke="#8a83bf"
                markerEnd="url(#sm-arrow)"
              />
              <text
                x={lane + 22}
                y={y1}
                dominantBaseline="middle"
                fill="#8a83bf"
                fontSize="10"
              >
                {label(t)}
              </text>
            </g>
          );
        }
        const d = `M ${String(right)} ${String(y1)} C ${String(lane)} ${String(y1)}, ${String(lane)} ${String(y2)}, ${String(right)} ${String(y2)}`;
        return (
          <g key={t.id}>
            <path
              d={d}
              fill="none"
              stroke="#8a83bf"
              markerEnd="url(#sm-arrow)"
            />
            <text
              x={lane + 6}
              y={(y1 + y2) / 2}
              dominantBaseline="middle"
              fill="#8a83bf"
              fontSize="10"
            >
              {label(t)}
            </text>
          </g>
        );
      })}
      {states.map((s, i) => (
        <g key={s.id}>
          <rect
            x={pad}
            y={pad + i * gapY}
            width={boxW}
            height={boxH}
            rx={8}
            fill="#1a1334"
            stroke={s.is_initial ? "#3dffa6" : "#2a1f52"}
            strokeWidth={s.is_initial ? 2 : 1}
          />
          {s.is_terminal && (
            <rect
              x={pad + 3}
              y={pad + i * gapY + 3}
              width={boxW - 6}
              height={boxH - 6}
              rx={6}
              fill="none"
              stroke="#2a1f52"
            />
          )}
          <text
            x={pad + 12}
            y={midY(i)}
            dominantBaseline="middle"
            fill="#ece6ff"
            fontSize="13"
          >
            {s.name}
          </text>
          {s.is_initial && (
            <text
              x={right - 10}
              y={midY(i)}
              dominantBaseline="middle"
              textAnchor="end"
              fill="#3dffa6"
              fontSize="9"
            >
              ▶ start
            </text>
          )}
        </g>
      ))}
    </svg>
  );
}

function TransitionTable({
  states,
  transitions,
}: {
  states: State[];
  transitions: Transition[];
}): React.JSX.Element {
  const nameOf = (key: string): string =>
    states.find((s) => s.stable_key === key)?.name ?? key;
  return (
    <table className="mt-3 w-full border-collapse text-[11px]">
      <thead>
        <tr className="text-text-dim">
          <th className="border-b border-border py-1 text-left font-semibold">
            From
          </th>
          <th className="border-b border-border py-1 text-left font-semibold">
            Event
          </th>
          <th className="border-b border-border py-1 text-left font-semibold">
            Guard
          </th>
          <th className="border-b border-border py-1 text-left font-semibold">
            To
          </th>
        </tr>
      </thead>
      <tbody>
        {transitions.map((t) => (
          <tr key={t.id} className="text-text">
            <td className="border-b border-border py-1 font-mono">
              {nameOf(t.from_state)}
            </td>
            <td className="border-b border-border py-1">{t.event || "—"}</td>
            <td className="border-b border-border py-1 text-text-dim">
              {t.guard ?? ""}
            </td>
            <td className="border-b border-border py-1 font-mono">
              {nameOf(t.to_state)}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

// Dedicated renderer for highly structured packet / struct layouts. Each field
// is (type, offset, size); rows are shown in offset order with a derived byte
// range so the layout reads like a wire-format table.
function FieldTable({
  fields,
  onEdit,
  onDelete,
}: {
  fields: Field[];
  onEdit: (f: Field) => void;
  onDelete: (f: Field) => void;
}): React.JSX.Element {
  const range = (f: Field): string => {
    if (f.offset == null) return "—";
    if (f.size == null) return String(f.offset);
    if (f.size <= 1) return String(f.offset);
    return `${String(f.offset)}–${String(f.offset + f.size - 1)}`;
  };
  return (
    <table className="w-full border-collapse text-[11px]">
      <thead>
        <tr className="text-text-dim">
          <th className="w-20 border-b border-border py-1 text-left font-semibold">
            Bytes
          </th>
          <th className="border-b border-border py-1 text-left font-semibold">
            Field
          </th>
          <th className="border-b border-border py-1 text-left font-semibold">
            Type
          </th>
          <th className="w-12 border-b border-border py-1 text-right font-semibold">
            Size
          </th>
          <th className="border-b border-border py-1 text-left font-semibold">
            Description
          </th>
          <th className="w-16 border-b border-border py-1" />
        </tr>
      </thead>
      <tbody>
        {fields.map((f) => (
          <tr key={f.id} className="align-top text-text">
            <td className="border-b border-border py-1 font-mono text-text-dim">
              {range(f)}
            </td>
            <td className="border-b border-border py-1 font-mono text-text-bright">
              {f.name}
            </td>
            <td className="border-b border-border py-1 font-mono text-info">
              {f.field_type || "—"}
            </td>
            <td className="border-b border-border py-1 text-right font-mono text-text-dim">
              {f.size == null ? "—" : String(f.size)}
            </td>
            <td className="border-b border-border py-1 text-text-dim">
              {f.description}
            </td>
            <td className="border-b border-border py-1 text-right">
              <Actions
                onEdit={(): void => {
                  onEdit(f);
                }}
                onDelete={(): void => {
                  onDelete(f);
                }}
              />
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

const confidenceColor: Record<string, string> = {
  high: "text-low",
  medium: "text-medium",
  low: "text-text-dim",
};

const claimStatusColor: Record<string, string> = {
  supported: "text-low",
  hypothesis: "text-medium",
  refuted: "text-critical",
};

const priorityColor: Record<string, string> = {
  high: "text-high",
  medium: "text-medium",
  low: "text-low",
};

function Badge({
  label,
  className,
}: {
  label: string;
  className?: string;
}): React.JSX.Element {
  return (
    <span
      className={`rounded-sm border border-border px-1.5 py-0.5 text-[9px] font-semibold tracking-wider uppercase ${className ?? "text-text-dim"}`}
    >
      {label}
    </span>
  );
}

function evidenceLabel(e: EvidenceLink): string {
  if (e.external_locator) {
    return e.external_kind
      ? `${e.external_kind}: ${e.external_locator}`
      : e.external_locator;
  }
  if (e.source_entity_id) {
    const kind = e.source_entity_type ?? "entity";
    return `${kind}:${e.source_entity_id.slice(0, 8)}`;
  }
  return "—";
}

function Actions({
  onEdit,
  onDelete,
  extra,
}: {
  onEdit?: () => void;
  onDelete: () => void;
  extra?: { label: string; onClick: () => void };
}): React.JSX.Element {
  return (
    <span className="flex shrink-0 gap-2 text-[10px]">
      {extra && (
        <button
          type="button"
          className="text-text-dim hover:text-accent"
          onClick={extra.onClick}
        >
          {extra.label}
        </button>
      )}
      {onEdit && (
        <button
          type="button"
          className="text-text-dim hover:text-accent"
          onClick={onEdit}
        >
          edit
        </button>
      )}
      <button
        type="button"
        className="text-text-dim hover:text-critical"
        onClick={onDelete}
      >
        delete
      </button>
    </span>
  );
}

function ClaimRow({
  claim,
  evidence,
  onEdit,
  onDelete,
  onAddEvidence,
}: {
  claim: Claim;
  evidence: EvidenceLink[];
  onEdit: () => void;
  onDelete: () => void;
  onAddEvidence: () => void;
}): React.JSX.Element {
  const backing = evidence.filter(
    (e) => e.target_type === "claim" && e.target_id === claim.id,
  );
  return (
    <div className="border-b border-border py-2 last:border-0">
      <div className="flex items-start justify-between gap-2">
        <p className="text-[13px] text-text">{claim.text}</p>
        <div className="flex shrink-0 items-center gap-1">
          <Badge
            label={claim.status}
            className={claimStatusColor[claim.status] ?? "text-text-dim"}
          />
          <Badge
            label={claim.confidence}
            className={confidenceColor[claim.confidence] ?? "text-text-dim"}
          />
          <Actions
            onEdit={onEdit}
            onDelete={onDelete}
            extra={{ label: "+ evidence", onClick: onAddEvidence }}
          />
        </div>
      </div>
      <div className="mt-1 flex flex-wrap items-center gap-2 text-[10px] text-text-dim">
        {backing.length === 0 ? (
          <span className="text-critical">no evidence</span>
        ) : (
          backing.map((e) => (
            <span key={e.id} className="font-mono text-info">
              ◆ {evidenceLabel(e)}
            </span>
          ))
        )}
      </div>
    </div>
  );
}

function QuestionRow({
  q,
  onEdit,
  onDelete,
}: {
  q: OpenQuestion;
  onEdit: () => void;
  onDelete: () => void;
}): React.JSX.Element {
  return (
    <div className="flex items-start justify-between gap-2 border-b border-border py-1.5 last:border-0">
      <p className="text-[13px] text-text">{q.question}</p>
      <div className="flex shrink-0 items-center gap-1">
        <Badge
          label={q.priority}
          className={priorityColor[q.priority] ?? "text-text-dim"}
        />
        {q.status !== "open" && <Badge label={q.status} />}
        <Actions onEdit={onEdit} onDelete={onDelete} />
      </div>
    </div>
  );
}

function SectionHeader({
  title,
  addLabel,
  onAdd,
}: {
  title: string;
  addLabel: string;
  onAdd: () => void;
}): React.JSX.Element {
  return (
    <div className="mb-1 flex items-center gap-2">
      <h2 className="text-[10px] font-semibold tracking-widest text-text-dim uppercase">
        {title}
      </h2>
      <button
        type="button"
        className="text-[10px] text-accent hover:underline"
        onClick={onAdd}
      >
        {addLabel}
      </button>
    </div>
  );
}

function Detail({ detail }: { detail: ExplanationDetail }): React.JSX.Element {
  const items = useStore((s) => s.items);
  const openTab = useStore((s) => s.openTab);
  const showExplanations = useStore((s) => s.showExplanations);
  const openForm = useStore((s) => s.openForm);
  const openConfirm = useStore((s) => s.openConfirm);

  const eid = detail.id;
  const nameFor = (id: string): string =>
    items.find((i) => i.item.id === id)?.item.name ?? id.slice(0, 8);
  const stateName = (key: string): string =>
    detail.states.find((s) => s.stable_key === key)?.name ?? key;
  const explEvidence = detail.evidence.filter(
    (e) => e.target_type === "explanation",
  );
  const isStateMachine =
    detail.explanation_type === "state_machine" || detail.states.length > 0;

  return (
    <div className="h-full overflow-y-auto p-5">
      <button
        type="button"
        onClick={showExplanations}
        className="mb-3 text-[11px] text-text-dim transition-colors hover:text-accent"
      >
        ← All explanations
      </button>

      <div className="mb-1 flex items-center gap-2">
        <h1 className="text-lg font-semibold text-text-bright">
          {detail.title}
        </h1>
        <Badge label={detail.explanation_type} className="text-accent" />
        <Badge label={detail.status} />
        <Badge
          label={`${detail.confidence} confidence`}
          className={confidenceColor[detail.confidence] ?? "text-text-dim"}
        />
        <Actions
          onEdit={(): void => {
            openForm(explanationEditForm(detail));
          }}
          onDelete={(): void => {
            openConfirm({
              title: "Delete explanation",
              message: `Delete "${detail.title}" and all its claims, questions, states, and evidence?`,
              tool: "explanation_delete",
              args: { id: eid },
            });
            showExplanations();
          }}
        />
      </div>

      {detail.scope_item_ids.length > 0 && (
        <div className="mb-3 flex flex-wrap items-center gap-1.5 text-[11px] text-text-dim">
          <span>Explains:</span>
          {detail.scope_item_ids.map((id) => (
            <button
              key={id}
              type="button"
              onClick={(): void => {
                openTab(id);
              }}
              className="font-mono text-info transition-colors hover:text-accent"
            >
              {nameFor(id)}
            </button>
          ))}
        </div>
      )}

      {detail.summary && (
        <p className="mb-4 max-w-prose text-[13px] whitespace-pre-wrap text-text">
          {detail.summary}
        </p>
      )}

      {isStateMachine && (
        <section className="mb-5">
          <SectionHeader
            title={`State machine (${String(detail.states.length)} states, ${String(detail.transitions.length)} transitions)`}
            addLabel="+ State"
            onAdd={(): void => {
              openForm(stateCreateForm(eid));
            }}
          />
          <StateMachineDiagram
            states={detail.states}
            transitions={detail.transitions}
          />
          <TransitionTable
            states={detail.states}
            transitions={detail.transitions}
          />
          <div className="mt-2 space-y-0.5">
            {detail.states.map((s) => (
              <div
                key={s.id}
                className="flex items-center gap-2 text-[11px] text-text-dim"
              >
                <span className="font-mono text-text">{s.name}</span>
                {s.is_initial && <span className="text-low">initial</span>}
                {s.is_terminal && <span>terminal</span>}
                <Actions
                  onEdit={(): void => {
                    openForm(stateEditForm(s));
                  }}
                  onDelete={(): void => {
                    openConfirm({
                      title: "Delete state",
                      message: `Delete state "${s.name}" and its transitions?`,
                      tool: "state_delete",
                      args: { id: s.id },
                    });
                  }}
                />
              </div>
            ))}
          </div>
          <div className="mt-2 flex items-center gap-2">
            <span className="text-[10px] tracking-widest text-text-dim uppercase">
              Transitions
            </span>
            <button
              type="button"
              className="text-[10px] text-accent hover:underline"
              onClick={(): void => {
                openForm(transitionCreateForm(eid, detail.states));
              }}
            >
              + Transition
            </button>
          </div>
          {detail.transitions.map((t) => (
            <div
              key={t.id}
              className="flex items-center gap-2 text-[11px] text-text-dim"
            >
              <span className="font-mono text-text">
                {stateName(t.from_state)} →{t.event ? ` ${t.event}` : ""} →{" "}
                {stateName(t.to_state)}
              </span>
              <Actions
                onEdit={(): void => {
                  openForm(transitionEditForm(t, detail.states));
                }}
                onDelete={(): void => {
                  openConfirm({
                    title: "Delete transition",
                    message: "Delete this transition?",
                    tool: "transition_delete",
                    args: { id: t.id },
                  });
                }}
              />
            </div>
          ))}
        </section>
      )}

      {(detail.fields.length > 0 ||
        detail.explanation_type === "packet_format" ||
        detail.explanation_type === "memory_layout") && (
        <section className="mb-5">
          <SectionHeader
            title={`Structure (${String(detail.fields.length)} fields)`}
            addLabel="+ Field"
            onAdd={(): void => {
              openForm(fieldCreateForm(eid));
            }}
          />
          {detail.fields.length > 0 ? (
            <FieldTable
              fields={detail.fields}
              onEdit={(f): void => {
                openForm(fieldEditForm(f));
              }}
              onDelete={(f): void => {
                openConfirm({
                  title: "Delete field",
                  message: `Delete field "${f.name}"?`,
                  tool: "field_delete",
                  args: { id: f.id },
                });
              }}
            />
          ) : (
            <p className="text-[12px] text-text-dim">No fields yet.</p>
          )}
        </section>
      )}

      {detail.diagram_html != null && detail.diagram_html !== "" && (
        <section className="mb-5">
          <h2 className="mb-1 text-[10px] font-semibold tracking-widest text-text-dim uppercase">
            Diagram
          </h2>
          {/* Server-sanitized HTML (ammonia): scripts, event handlers, and
              unsafe URLs are stripped before storage, so this is safe to
              render. CSP (default-src 'self') is the second layer. */}
          <div
            className="lsvr-html-diagram overflow-x-auto rounded-sm border border-border bg-surface p-3 text-[12px] text-text"
            dangerouslySetInnerHTML={{ __html: detail.diagram_html }}
          />
        </section>
      )}

      <section className="mb-5">
        <SectionHeader
          title={`Claims (${String(detail.claims.length)})`}
          addLabel="+ Claim"
          onAdd={(): void => {
            openForm(claimCreateForm(eid));
          }}
        />
        {detail.claims.length === 0 ? (
          <p className="text-[12px] text-text-dim">No claims recorded.</p>
        ) : (
          detail.claims.map((c) => (
            <ClaimRow
              key={c.id}
              claim={c}
              evidence={detail.evidence}
              onEdit={(): void => {
                openForm(claimEditForm(c));
              }}
              onAddEvidence={(): void => {
                openForm(evidenceCreateForm("claim", c.id));
              }}
              onDelete={(): void => {
                openConfirm({
                  title: "Delete claim",
                  message: "Delete this claim?",
                  tool: "claim_delete",
                  args: { id: c.id },
                });
              }}
            />
          ))
        )}
      </section>

      <section className="mb-5">
        <SectionHeader
          title={`Open Questions (${String(detail.open_questions.filter((q) => q.status === "open").length)})`}
          addLabel="+ Question"
          onAdd={(): void => {
            openForm(questionCreateForm(eid));
          }}
        />
        {detail.open_questions.length === 0 ? (
          <p className="text-[12px] text-text-dim">No open questions.</p>
        ) : (
          detail.open_questions.map((q) => (
            <QuestionRow
              key={q.id}
              q={q}
              onEdit={(): void => {
                openForm(questionEditForm(q));
              }}
              onDelete={(): void => {
                openConfirm({
                  title: "Delete open question",
                  message: "Delete this question?",
                  tool: "open_question_delete",
                  args: { id: q.id },
                });
              }}
            />
          ))
        )}
      </section>

      <section>
        <SectionHeader
          title="Evidence"
          addLabel="+ Evidence"
          onAdd={(): void => {
            openForm(evidenceCreateForm("explanation", eid));
          }}
        />
        {explEvidence.length === 0 ? (
          <p className="text-[12px] text-text-dim">
            No explanation-level evidence.
          </p>
        ) : (
          explEvidence.map((e) => (
            <div
              key={e.id}
              className="flex items-start gap-2 border-b border-border py-1.5 text-[12px] last:border-0"
            >
              <div className="flex-1">
                <span className="font-mono text-info">
                  ◆ {evidenceLabel(e)}
                </span>{" "}
                <span className="text-text-dim">
                  ({e.evidence_type}, {e.strength})
                </span>
                {e.excerpt && (
                  <p className="mt-0.5 text-[11px] text-text-dim italic">
                    {e.excerpt}
                  </p>
                )}
              </div>
              <Actions
                onDelete={(): void => {
                  openConfirm({
                    title: "Delete evidence",
                    message: "Delete this evidence link?",
                    tool: "evidence_delete",
                    args: { id: e.id },
                  });
                }}
              />
            </div>
          ))
        )}
      </section>
    </div>
  );
}

export function Explanations(): React.JSX.Element {
  const explanations = useStore((s) => s.explanations);
  const explanationDetails = useStore((s) => s.explanationDetails);
  const selected = useStore((s) => s.selectedExplanation);
  const openExplanation = useStore((s) => s.openExplanation);
  const openForm = useStore((s) => s.openForm);

  if (selected) {
    const detail = explanationDetails[selected];
    if (detail) return <Detail detail={detail} />;
  }

  return (
    <div className="h-full overflow-y-auto p-5">
      <div className="mb-1 flex items-center gap-3">
        <h1 className="text-lg font-semibold text-text-bright">Explanations</h1>
        <button
          type="button"
          className="text-[11px] text-accent hover:underline"
          onClick={(): void => {
            openForm(explanationCreateForm());
          }}
        >
          + New explanation
        </button>
      </div>
      <p className="mb-4 text-[12px] text-text-dim">
        Evidence-backed models of how the target works — humans and agents share
        the same CRUD.
      </p>
      {explanations.length === 0 ? (
        <p className="text-[12px] text-text-dim">
          No explanations yet — create one, or ask an agent to{" "}
          <code>explanation_upsert</code> a model of a subsystem.
        </p>
      ) : (
        <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
          {explanations.map((e) => (
            <button
              key={e.id}
              type="button"
              onClick={(): void => {
                openExplanation(e.id);
              }}
              className="rounded-sm border border-border bg-surface p-3 text-left transition-colors hover:border-accent hover:bg-surface-hover"
            >
              <div className="mb-1 flex items-center gap-2">
                <span className="flex-1 truncate text-[13px] font-semibold text-text-bright">
                  {e.title}
                </span>
                <Badge label={e.explanation_type} className="text-accent" />
              </div>
              {e.summary && (
                <p className="mb-2 line-clamp-2 text-[11px] text-text-dim">
                  {e.summary}
                </p>
              )}
              <div className="flex items-center gap-3 text-[10px] text-text-dim tabular-nums">
                <span>{e.claim_count} claims</span>
                <span>{e.open_question_count} open Q</span>
                <span>{e.evidence_count} evidence</span>
                <span
                  className={`ml-auto ${confidenceColor[e.confidence] ?? ""}`}
                >
                  {e.confidence}
                </span>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
