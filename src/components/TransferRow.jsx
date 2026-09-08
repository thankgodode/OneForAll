import React from "react";
import { ArrowUpRight, ArrowDownLeft, Pause, Play, X, RotateCcw, Trash2 } from "lucide-react";

// One row in the Transfers list. `onAction(id, action)` is called for every
// button press — the parent view owns the actual state transition logic.
export default function TransferRow({ t, onAction }) {
  const color = t.status === "failed" ? "var(--red)" : t.status === "completed" ? "var(--teal)" : "var(--blue)";
  return (
    <div className="card" style={{ padding: 14, display: "flex", alignItems: "center", gap: 14 }}>
      <div style={{ width: 34, height: 34, borderRadius: 9, background: `${color}1A`, color, display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
        {t.direction === "send" ? <ArrowUpRight size={16} /> : <ArrowDownLeft size={16} />}
      </div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 8 }}>
          <div style={{ fontSize: 13, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{t.file}</div>
          <div className="mono" style={{ fontSize: 11, color: "var(--text-tertiary)", flexShrink: 0 }}>{t.size}</div>
        </div>
        <div style={{ fontSize: 11.5, color: "var(--text-tertiary)", marginBottom: t.status === "active" ? 6 : 0 }}>
          {t.direction === "send" ? `To ${t.target}` : `From ${t.target}`}
          {t.status === "failed" && <span style={{ color: "var(--red)" }}> · {t.error}</span>}
        </div>
        {t.status === "active" && (
          <>
            <div className="progress-track">
              <div className="progress-fill" style={{ width: `${t.progress}%`, background: color }} />
            </div>
            <div style={{ display: "flex", justifyContent: "space-between", marginTop: 4 }}>
              <span className="mono" style={{ fontSize: 10.5, color: "var(--text-tertiary)" }}>{t.progress}% · {t.speed}</span>
              <span className="mono" style={{ fontSize: 10.5, color: "var(--text-tertiary)" }}>{t.eta} left</span>
            </div>
          </>
        )}
      </div>
      <div style={{ display: "flex", gap: 6, flexShrink: 0 }}>
        {t.status === "active" && <button className="btn-icon" onClick={() => onAction(t.id, "pause")} title="Pause"><Pause size={13} /></button>}
        {t.status === "failed" && <button className="btn-icon" onClick={() => onAction(t.id, "resume")} title="Resume from where it left off"><Play size={13} /></button>}
        {t.status === "failed" && <button className="btn-icon" onClick={() => onAction(t.id, "retry")} title="Start over"><RotateCcw size={13} /></button>}
        {t.status === "active" && <button className="btn-icon" onClick={() => onAction(t.id, "cancel")} title="Cancel"><X size={13} /></button>}
        {(t.status === "completed" || t.status === "failed") && <button className="btn-icon" onClick={() => onAction(t.id, "remove")} title="Remove"><Trash2 size={13} /></button>}
      </div>
    </div>
  );
}
