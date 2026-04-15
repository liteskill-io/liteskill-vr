const severityStyles: Record<string, string> = {
  critical: "bg-critical/20 text-critical",
  high: "bg-high/20 text-high",
  medium: "bg-medium/20 text-medium",
  low: "bg-low/20 text-low",
  info: "bg-info/20 text-info",
};

const statusStyles: Record<string, string> = {
  draft: "border-draft text-draft",
  confirmed: "border-confirmed text-confirmed",
  false_positive: "border-false-positive text-false-positive line-through",
  reported: "border-accent text-accent",
  fixed: "border-low text-low",
};

export function SeverityBadge({
  severity,
}: {
  severity: string | undefined;
}): React.JSX.Element | null {
  if (!severity) return null;
  return (
    <span
      className={`rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase ${severityStyles[severity] ?? "bg-text-dim/20 text-text-dim"}`}
    >
      {severity}
    </span>
  );
}

export function StatusBadge({ status }: { status: string }): React.JSX.Element {
  return (
    <span
      className={`rounded border px-1.5 py-0.5 text-[10px] ${statusStyles[status] ?? "border-text-dim text-text-dim"}`}
    >
      {status.replace("_", " ")}
    </span>
  );
}
