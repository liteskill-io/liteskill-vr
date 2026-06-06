import { useStore } from "@/lib/store";

import type {
  Claim,
  EvidenceLink,
  ExplanationDetail,
  OpenQuestion,
} from "@/lib/types";

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

function ClaimRow({
  claim,
  evidence,
}: {
  claim: Claim;
  evidence: EvidenceLink[];
}): React.JSX.Element {
  const backing = evidence.filter(
    (e) => e.target_type === "claim" && e.target_id === claim.id,
  );
  return (
    <div className="border-b border-border py-2 last:border-0">
      <div className="flex items-start justify-between gap-2">
        <p className="text-[13px] text-text">{claim.text}</p>
        <div className="flex shrink-0 gap-1">
          <Badge
            label={claim.status}
            className={claimStatusColor[claim.status] ?? "text-text-dim"}
          />
          <Badge
            label={claim.confidence}
            className={confidenceColor[claim.confidence] ?? "text-text-dim"}
          />
        </div>
      </div>
      <div className="mt-1 flex flex-wrap items-center gap-2 text-[10px] text-text-dim">
        <span className="font-mono">{claim.stable_key}</span>
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

function QuestionRow({ q }: { q: OpenQuestion }): React.JSX.Element {
  return (
    <div className="flex items-start justify-between gap-2 border-b border-border py-1.5 last:border-0">
      <p className="text-[13px] text-text">{q.question}</p>
      <div className="flex shrink-0 gap-1">
        <Badge
          label={q.priority}
          className={priorityColor[q.priority] ?? "text-text-dim"}
        />
        {q.status !== "open" && <Badge label={q.status} />}
      </div>
    </div>
  );
}

function Detail({ detail }: { detail: ExplanationDetail }): React.JSX.Element {
  const items = useStore((s) => s.items);
  const openTab = useStore((s) => s.openTab);
  const showExplanations = useStore((s) => s.showExplanations);

  const nameFor = (id: string): string =>
    items.find((i) => i.item.id === id)?.item.name ?? id.slice(0, 8);

  const explEvidence = detail.evidence.filter(
    (e) => e.target_type === "explanation",
  );

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

      <section className="mb-5">
        <h2 className="mb-1 text-[10px] font-semibold tracking-widest text-text-dim uppercase">
          Claims ({detail.claims.length})
        </h2>
        {detail.claims.length === 0 ? (
          <p className="text-[12px] text-text-dim">No claims recorded.</p>
        ) : (
          detail.claims.map((c) => (
            <ClaimRow key={c.id} claim={c} evidence={detail.evidence} />
          ))
        )}
      </section>

      <section className="mb-5">
        <h2 className="mb-1 text-[10px] font-semibold tracking-widest text-text-dim uppercase">
          Open Questions (
          {detail.open_questions.filter((q) => q.status === "open").length})
        </h2>
        {detail.open_questions.length === 0 ? (
          <p className="text-[12px] text-text-dim">No open questions.</p>
        ) : (
          detail.open_questions.map((q) => <QuestionRow key={q.id} q={q} />)
        )}
      </section>

      {explEvidence.length > 0 && (
        <section>
          <h2 className="mb-1 text-[10px] font-semibold tracking-widest text-text-dim uppercase">
            Evidence
          </h2>
          {explEvidence.map((e) => (
            <div
              key={e.id}
              className="border-b border-border py-1.5 text-[12px] last:border-0"
            >
              <span className="font-mono text-info">◆ {evidenceLabel(e)}</span>{" "}
              <span className="text-text-dim">
                ({e.evidence_type}, {e.strength})
              </span>
              {e.excerpt && (
                <p className="mt-0.5 text-[11px] text-text-dim italic">
                  {e.excerpt}
                </p>
              )}
            </div>
          ))}
        </section>
      )}
    </div>
  );
}

export function Explanations(): React.JSX.Element {
  const explanations = useStore((s) => s.explanations);
  const explanationDetails = useStore((s) => s.explanationDetails);
  const selected = useStore((s) => s.selectedExplanation);
  const openExplanation = useStore((s) => s.openExplanation);

  if (selected) {
    const detail = explanationDetails[selected];
    if (detail) return <Detail detail={detail} />;
  }

  return (
    <div className="h-full overflow-y-auto p-5">
      <h1 className="mb-1 text-lg font-semibold text-text-bright">
        Explanations
      </h1>
      <p className="mb-4 text-[12px] text-text-dim">
        Evidence-backed models of how the target works. Created by agents over
        MCP; read-only here.
      </p>
      {explanations.length === 0 ? (
        <p className="text-[12px] text-text-dim">
          No explanations yet. Ask an agent to <code>explanation_upsert</code> a
          model of a subsystem.
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
