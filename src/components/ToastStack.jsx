import React from "react";
import { Check, AlertTriangle } from "lucide-react";

// Renders the active toast queue, bottom-right and just above the status
// bar — small and quiet, closer to a native OS notification than a
// colorful SaaS banner.
export default function ToastStack({ toasts }) {
  return (
    <div style={{ position: "absolute", bottom: 34, right: 14, display: "flex", flexDirection: "column-reverse", gap: 6, zIndex: 100, width: 250 }}>
      {toasts.map((t) => (
        <div key={t.id} className="toast">
          <div style={{ marginTop: 1, color: t.error ? "var(--red)" : "var(--teal)" }}>
            {t.error ? <AlertTriangle size={13} /> : <Check size={13} />}
          </div>
          <div style={{ color: "var(--text-secondary)", lineHeight: 1.4 }}>{t.message}</div>
        </div>
      ))}
    </div>
  );
}
