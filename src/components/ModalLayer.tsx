import { useEffect, useState } from "react";

import { mcpCall } from "@/lib/ipc";
import { useStore } from "@/lib/store";

import type { FormDesc, FormField } from "@/lib/store";

type FieldValue = string | boolean;

function initialValues(form: FormDesc): Record<string, FieldValue> {
  const out: Record<string, FieldValue> = {};
  for (const f of form.fields) {
    const init = form.initial?.[f.name];
    if (f.type === "checkbox") {
      out[f.name] = init === true;
    } else if (f.type === "tags") {
      out[f.name] = Array.isArray(init) ? init.join(", ") : "";
    } else {
      out[f.name] =
        typeof init === "string" || typeof init === "number"
          ? String(init)
          : "";
    }
  }
  return out;
}

// Convert form values into mcp_call args: hidden values first, then fields.
// Empty optional strings are omitted so they don't overwrite on update / so
// optional params stay unset on create.
function buildArgs(
  form: FormDesc,
  values: Record<string, FieldValue>,
): Record<string, unknown> {
  const args: Record<string, unknown> = { ...form.hidden };
  for (const f of form.fields) {
    const v = values[f.name];
    if (f.type === "checkbox") {
      args[f.name] = v === true;
    } else if (f.type === "tags") {
      const list = String(v ?? "")
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
      if (list.length > 0) args[f.name] = list;
    } else if (f.type === "number") {
      const s = String(v ?? "").trim();
      const n = Number(s);
      if (s !== "" && !Number.isNaN(n)) args[f.name] = n;
    } else {
      const s = String(v ?? "").trim();
      if (s !== "") args[f.name] = s;
    }
  }
  return args;
}

function Field({
  field,
  value,
  onChange,
}: {
  field: FormField;
  value: FieldValue;
  onChange: (v: FieldValue) => void;
}): React.JSX.Element {
  const base =
    "w-full rounded-sm border border-border bg-bg px-2 py-1 text-[13px] text-text focus:border-accent focus:outline-none";
  return (
    <label className="mb-2 block">
      <span className="mb-0.5 block text-[10px] font-semibold tracking-wide text-text-dim uppercase">
        {field.label}
        {field.required && <span className="text-critical"> *</span>}
      </span>
      {field.type === "textarea" ? (
        <textarea
          className={`${base} h-20 resize-y font-mono`}
          value={String(value)}
          placeholder={field.placeholder}
          onChange={(e): void => {
            onChange(e.target.value);
          }}
        />
      ) : field.type === "select" ? (
        <select
          className={base}
          value={String(value)}
          onChange={(e): void => {
            onChange(e.target.value);
          }}
        >
          <option value="">—</option>
          {field.options?.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      ) : field.type === "checkbox" ? (
        <input
          type="checkbox"
          checked={value === true}
          onChange={(e): void => {
            onChange(e.target.checked);
          }}
        />
      ) : (
        <input
          type={field.type === "number" ? "number" : "text"}
          className={base}
          value={String(value)}
          placeholder={field.placeholder}
          onChange={(e): void => {
            onChange(e.target.value);
          }}
        />
      )}
    </label>
  );
}

function ToolForm({ form }: { form: FormDesc }): React.JSX.Element {
  const closeForm = useStore((s) => s.closeForm);
  const [values, setValues] = useState<Record<string, FieldValue>>(() =>
    initialValues(form),
  );
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (): Promise<void> => {
    // Client-side required check (the backend validates authoritatively).
    for (const f of form.fields) {
      if (f.required && String(values[f.name] ?? "").trim() === "") {
        setError(`${f.label} is required`);
        return;
      }
    }
    setBusy(true);
    setError(null);
    try {
      await mcpCall(form.tool, buildArgs(form, values));
      closeForm();
    } catch (e) {
      setError(typeof e === "string" ? e : String(e));
      setBusy(false);
    }
  };

  return (
    <Overlay onClose={closeForm}>
      <h2 className="mb-3 text-[13px] font-semibold text-text-bright">
        {form.title}
      </h2>
      {form.fields.map((f) => (
        <Field
          key={f.name}
          field={f}
          value={values[f.name] ?? ""}
          onChange={(v): void => {
            setValues((prev) => ({ ...prev, [f.name]: v }));
          }}
        />
      ))}
      {error != null && (
        <p className="mt-1 text-[11px] text-critical">{error}</p>
      )}
      <div className="mt-3 flex justify-end gap-2">
        <button
          type="button"
          className="rounded-sm px-3 py-1 text-[12px] text-text-dim hover:text-text"
          onClick={closeForm}
        >
          Cancel
        </button>
        <button
          type="button"
          disabled={busy}
          className="rounded-sm bg-accent px-3 py-1 text-[12px] font-semibold text-bg disabled:opacity-50"
          onClick={(): void => {
            void submit();
          }}
        >
          {form.submitLabel ?? "Save"}
        </button>
      </div>
    </Overlay>
  );
}

function ConfirmDialog(): React.JSX.Element | null {
  const confirm = useStore((s) => s.confirm);
  const closeConfirm = useStore((s) => s.closeConfirm);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  if (!confirm) return null;

  const run = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      await mcpCall(confirm.tool, confirm.args);
      closeConfirm();
    } catch (e) {
      setError(typeof e === "string" ? e : String(e));
      setBusy(false);
    }
  };

  return (
    <Overlay onClose={closeConfirm}>
      <h2 className="mb-2 text-[13px] font-semibold text-text-bright">
        {confirm.title}
      </h2>
      <p className="text-[12px] text-text">{confirm.message}</p>
      {error != null && (
        <p className="mt-1 text-[11px] text-critical">{error}</p>
      )}
      <div className="mt-3 flex justify-end gap-2">
        <button
          type="button"
          className="rounded-sm px-3 py-1 text-[12px] text-text-dim hover:text-text"
          onClick={closeConfirm}
        >
          Cancel
        </button>
        <button
          type="button"
          disabled={busy}
          className="rounded-sm bg-critical px-3 py-1 text-[12px] font-semibold text-bg disabled:opacity-50"
          onClick={(): void => {
            void run();
          }}
        >
          Delete
        </button>
      </div>
    </Overlay>
  );
}

function Overlay({
  children,
  onClose,
}: {
  children: React.ReactNode;
  onClose: () => void;
}): React.JSX.Element {
  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return (): void => {
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center p-8">
      <button
        type="button"
        aria-label="Close"
        className="absolute inset-0 cursor-default bg-black/50"
        onClick={onClose}
      />
      <div
        className="relative max-h-[80vh] w-[28rem] overflow-y-auto rounded-md border border-border bg-surface p-4 shadow-xl"
        role="dialog"
        aria-modal="true"
      >
        {children}
      </div>
    </div>
  );
}

// Single global modal layer; mounted once at the app root.
export function ModalLayer(): React.JSX.Element | null {
  const activeForm = useStore((s) => s.activeForm);
  const confirm = useStore((s) => s.confirm);
  if (activeForm) return <ToolForm form={activeForm} />;
  if (confirm) return <ConfirmDialog />;
  return null;
}
