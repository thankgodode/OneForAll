import React from "react";
import { Wifi, WifiOff } from "lucide-react";

// Persistent bottom status strip, visible regardless of which view is
// active — the pattern used by Finder, VS Code, Transmission, Docker
// Desktop, etc. Carries ambient state (network, device count, local IP)
// so it doesn't need to live in every view's header.
export default function StatusBar({ network, devices, transfers }) {
  const online = devices.filter((d) => !d.isSelf && d.status !== "offline").length;
  const active = transfers.filter((t) => t.status === "active").length;
  return (
    <div className="statusbar mono">
      <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
        <span style={{ display: "flex", alignItems: "center", gap: 5, color: network.connected ? "var(--teal)" : "var(--text-tertiary)" }}>
          {network.connected ? <Wifi size={11} /> : <WifiOff size={11} />}
          {network.connected ? network.sessionName : "Not connected"}
        </span>
        <span>{online} device{online !== 1 ? "s" : ""} online</span>
        {active > 0 && <span style={{ color: "var(--blue)" }}>{active} transferring</span>}
      </div>
      <div>192.168.1.42</div>
    </div>
  );
}
