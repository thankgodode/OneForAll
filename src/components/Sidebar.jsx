import React from "react";
import { Radio, Wifi, WifiOff } from "lucide-react";
import Avatar from "./Avatar";
import { NAV } from "../data/nav";

// Left navigation rail. Fully controlled: `view`/`setView` decide what's active,
// `profile`/`network` decide what's shown in the footer identity block.
export default function Sidebar({ view, setView, profile, network }) {
  return (
    <div style={{ width: 224, borderRight: "1px solid var(--border)", background: "var(--bg-elevated)", display: "flex", flexDirection: "column", padding: "18px 14px", flexShrink: 0 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 9, padding: "0 8px", marginBottom: 26 }}>
        <div style={{ width: 26, height: 26, borderRadius: 7, background: "var(--teal-dim)", display: "flex", alignItems: "center", justifyContent: "center", color: "var(--teal)" }}>
          <Radio size={14} />
        </div>
        <div className="disp" style={{ fontSize: 15, fontWeight: 700, letterSpacing: "-.01em" }}>Nearby</div>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 2, marginLeft: 12 }}>
        {NAV.map((n) => (
          <button key={n.id} onClick={() => setView(n.id)} style={{ background: "none", border: "none", textAlign: "left", width: "100%", padding: 0 }}>
            <div className={`navlink ${view === n.id ? "active" : ""}`} style={{ width: "100%" }}>
              <n.icon size={15} /> {n.label}
            </div>
          </button>
        ))}
      </div>

      <div style={{ marginTop: "auto", padding: "12px 8px", borderTop: "1px solid var(--border)", display: "flex", alignItems: "center", gap: 10 }}>
        <Avatar device={{ name: profile.displayName, color: profile.avatarColor }} size={30} />
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: 12.5, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{profile.displayName}</div>
          <div style={{ fontSize: 10.5, color: network.connected ? "var(--teal)" : "var(--text-tertiary)", display: "flex", alignItems: "center", gap: 4 }}>
            {network.connected ? <Wifi size={10} /> : <WifiOff size={10} />} {network.connected ? "Connected" : "Offline"}
          </div>
        </div>
      </div>
    </div>
  );
}
