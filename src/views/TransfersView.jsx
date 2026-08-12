import React, { useState } from "react";
import { ArrowUpRight } from "lucide-react";
import TransferRow from "../components/TransferRow";

// Tabbed list of transfers (active/completed/failed). Tab selection is
// local UI state; the underlying transfer data and mutation logic live
// in the App via `onAction`.
export default function TransfersView({ transfers, onAction }) {
  const [tab, setTab] = useState("active");
  const matches = (t, tb) => (tb === "active" ? t.status === "active" || t.status === "paused" : t.status === tb);
  const filtered = transfers.filter((t) => matches(t, tab));

  return (
    <div className="fade-up" style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      <div style={{ display: "flex", gap: 20, borderBottom: "1px solid var(--border)" }}>
        {["active", "completed", "failed"].map((tb) => (
          <button
            key={tb}
            className={`tab ${tab === tb ? "active" : ""}`}
            style={{ background: "none", border: "none", borderBottom: tab === tb ? "2px solid var(--teal)" : "2px solid transparent" }}
            onClick={() => setTab(tb)}
          >
            {tb[0].toUpperCase() + tb.slice(1)}{" "}
            <span className="mono" style={{ color: "var(--text-tertiary)", fontWeight: 400 }}>
              ({transfers.filter((t) => matches(t, tb)).length})
            </span>
          </button>
        ))}
      </div>
      {filtered.length === 0 ? (
        <div className="empty-state">
          <ArrowUpRight size={22} />
          <div>Nothing here yet</div>
          <div style={{ fontSize: 12 }}>{tab === "active" ? "Files you send or receive will appear here." : `No ${tab} transfers.`}</div>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          {filtered.map((t) => <TransferRow key={t.id} t={t} onAction={onAction} />)}
        </div>
      )}
    </div>
  );
}
