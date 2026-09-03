import React from "react";
import { AlertTriangle } from "lucide-react";

// Generic confirmation dialog for destructive actions.
// `modal` shape: { title, body, confirmLabel?, onConfirm }. Pass null to hide.
export default function ConfirmModal({ modal, onClose }) {
  if (!modal) return null;
  return (
    <div style={{ position: "absolute", inset: 0, background: "rgba(6,9,10,0.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 200, backdropFilter: "blur(2px)" }}>
      <div className="card fade-up" style={{ width: 360, background: "var(--bg-elevated)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 10 }}>
          <div style={{ width: 32, height: 32, borderRadius: 8, background: "var(--red-dim)", color: "var(--red)", display: "flex", alignItems: "center", justifyContent: "center" }}>
            <AlertTriangle size={16} />
          </div>
          <div className="disp" style={{ fontSize: 15, fontWeight: 600 }}>{modal.title}</div>
        </div>
        <div style={{ fontSize: 13, color: "var(--text-secondary)", lineHeight: 1.5, marginBottom: 20 }}>{modal.body}</div>
        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
          <button className="btn btn-ghost" onClick={onClose}>Cancel</button>
          <button
            className="btn btn-danger"
            style={{ background: "var(--red)", color: "#2A0B0D", borderColor: "var(--red)" }}
            onClick={() => { modal.onConfirm(); onClose(); }}
          >
            {modal.confirmLabel || "Confirm"}
          </button>
        </div>
      </div>
    </div>
  );
}
