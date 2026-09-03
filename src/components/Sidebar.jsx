import React from "react";
import { Wifi, WifiOff } from "lucide-react";
import Avatar from "./Avatar";
import { NAV } from "../data/nav";

// Left navigation rail. Fully controlled: `view`/`setView` decide what's active,
// `profile`/`network` decide what's shown in the footer identity block.
export default function Sidebar({ view, setView, profile, network }) {
  return (
    <div style={{ width: 196, borderRight: "1px solid var(--border)", background: "var(--bg-elevated)", display: "flex", flexDirection: "column", padding: "10px 10px", flexShrink: 0 }}>
      <div className="nav-section-label" style={{ paddingTop: 4 }}>Nearby</div>

      <div style={{ display: "flex", flexDirection: "column", gap: 1, marginLeft: 11 }}>
        {NAV.map((n) => (
          <button key={n.id} onClick={() => setView(n.id)} style={{ background: "none", border: "none", textAlign: "left", width: "100%", padding: 0 }}>
            <div className={`navlink ${view === n.id ? "active" : ""}`} style={{ width: "100%" }}>
              <n.icon size={14} /> {n.label}
            </div>
          </button>
        ))}
      </div>

      <div style={{ marginTop: "auto", padding: "8px 6px", borderTop: "1px solid var(--border)", display: "flex", alignItems: "center", gap: 8 }}>
        <Avatar device={{ name: profile.displayName, color: profile.avatarColor }} size={26} />
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: 11.5, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{profile.displayName}</div>
          <div style={{ fontSize: 10, color: network.connected ? "var(--teal)" : "var(--text-tertiary)", display: "flex", alignItems: "center", gap: 4 }}>
            {network.connected ? <Wifi size={9} /> : <WifiOff size={9} />} {network.connected ? "Connected" : "Offline"}
          </div>
        </div>
      </div>
    </div>
  );
}
