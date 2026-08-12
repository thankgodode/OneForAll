import React from "react";
import { Plus } from "lucide-react";
import { VIEW_TITLES } from "../data/nav";

// Page header: title/subtitle driven by the current view, plus the
// current session chip and a global "Send" shortcut.
export default function TopBar({ view, network, onSend }) {
  const [title, subtitle] = VIEW_TITLES[view];
  return (
    <div style={{ padding: "20px 28px", borderBottom: "1px solid var(--border)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
      <div>
        <div className="disp" style={{ fontSize: 20, fontWeight: 700 }}>{title}</div>
        <div style={{ fontSize: 12.5, color: "var(--text-tertiary)", marginTop: 2 }}>{subtitle}</div>
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <span className="chip mono" style={{ background: "var(--teal-glow)", color: "var(--teal)" }}>{network.sessionName}</span>
        <button className="btn btn-primary" onClick={onSend}><Plus size={14} /> Send</button>
      </div>
    </div>
  );
}
