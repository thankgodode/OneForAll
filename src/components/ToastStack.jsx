import React from "react";
import { Check, AlertTriangle } from "lucide-react";

// Renders the active toast queue in the top-right corner.
// Toast lifecycle (add/auto-dismiss) is owned by the App, not this component.
export default function ToastStack({ toasts }) {
  return (
    <div style={{ position: "absolute", top: 20, right: 20, display: "flex", flexDirection: "column", gap: 8, zIndex: 100, width: 300 }}>
      {toasts.map((t) => (
        <div
          key={t.id}
          style={{
            animation: "toastIn .25s ease both",
            background: "var(--panel)",
            border: `1px solid ${t.error ? "var(--red-dim)" : "var(--border-strong)"}`,
            borderRadius: "var(--radius-md)",
            padding: "12px 14px",
            display: "flex",
            gap: 10,
            alignItems: "flex-start",
            boxShadow: "0 8px 24px rgba(0,0,0,0.35)",
          }}
        >
          <div style={{ marginTop: 1, color: t.error ? "var(--red)" : "var(--teal)" }}>
            {t.error ? <AlertTriangle size={16} /> : <Check size={16} />}
          </div>
          <div style={{ fontSize: 12.5, color: "var(--text-secondary)", lineHeight: 1.4 }}>{t.message}</div>
        </div>
      ))}
    </div>
  );
}
