import React from "react";
import { Plus } from "lucide-react";
import { VIEW_TITLES } from "../data/nav";

// Slim contextual toolbar (Finder/Mail-style) — just the current section
// name, the active session, and one quick action. No hero header, no
// subtitle copy: that pattern reads as a web dashboard, not a window.
export default function TopBar({ view, network, onSend }) {
  const [title] = VIEW_TITLES[view];
  return (
    <div className="toolbar">
      <span className="disp" style={{ fontSize: 12.5, fontWeight: 600, color: "var(--text-primary)" }}>{title}</span>
      <span style={{ flex: 1 }} />
      <span className="chip mono" style={{ background: "var(--teal-glow)", color: "var(--teal)" }}>{network.sessionName}</span>
      <button className="btn-icon" onClick={onSend} title="Send files"><Plus size={13} /></button>
    </div>
  );
}
