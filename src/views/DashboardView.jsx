import React from "react";
import { Radio, ArrowUpRight, ArrowDownLeft, Printer, Wifi, ChevronRight } from "lucide-react";
import DeviceRow from "../components/DeviceRow";
import StatusChip from "../components/StatusChip";
import MeshView from "../components/MeshView";

// Single divided strip instead of four separate floating stat cards —
// reads as one native status readout rather than a row of web-dashboard
// KPI widgets.
function StatusStrip({ online, active, jobs, network }) {
  const seg = (icon, label, value, color) => (
    <div className="strip-seg" style={{ display: "flex", alignItems: "center", gap: 9 }}>
      <div style={{ color }}>{icon}</div>
      <div>
        <div className="disp" style={{ fontSize: 15, fontWeight: 700, lineHeight: 1 }}>{value}</div>
        <div style={{ fontSize: 10, color: "var(--text-tertiary)", marginTop: 2 }}>{label}</div>
      </div>
    </div>
  );
  return (
    <div className="strip">
      {seg(<Radio size={14} />, "Devices online", online, "var(--teal)")}
      {seg(<ArrowUpRight size={14} />, "Active transfers", active, "var(--blue)")}
      {seg(<Printer size={14} />, "Print jobs", jobs, "var(--amber)")}
      {seg(<Wifi size={14} />, "Network", network.connected ? "Connected" : "Offline", network.connected ? "var(--teal)" : "var(--gray)")}
    </div>
  );
}

// Landing view: status strip, the mesh diagram, the device list, and a
// rollup of recent transfer activity. Reads devices/transfers/printJobs
// as plain props — no local data fetching happens here.
export default function DashboardView({ devices, transfers, printJobs, network, setView }) {
  const online = devices.filter((d) => !d.isSelf && d.status !== "offline").length;
  const active = transfers.filter((t) => t.status === "active").length;
  const jobs = printJobs.filter((p) => p.status === "printing" || p.status === "pending").length;

  return (
    <div className="fade-up" style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      <StatusStrip online={online} active={active} jobs={jobs} network={network} />

      <div className="card">
        <div className="panel-head">
          <div className="disp" style={{ fontSize: 13, fontWeight: 600 }}>Network topology</div>
          <button className="btn btn-ghost" onClick={() => setView("network")}>Manage <ChevronRight size={13} /></button>
        </div>
        <MeshView devices={devices} />
      </div>

      <div style={{ display: "flex", gap: 14, flexWrap: "wrap" }}>
        <div className="card" style={{ flex: 2, minWidth: 320 }}>
          <div className="panel-head">
            <div className="disp" style={{ fontSize: 13, fontWeight: 600 }}>Connected devices</div>
            <button className="btn btn-ghost" onClick={() => setView("send")}>Send files <ChevronRight size={13} /></button>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
            {devices.filter((d) => !d.isSelf).map((d) => (
              <DeviceRow
                key={d.id}
                device={d}
                trailing={
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    {d.hasPrinter && <Printer size={13} color="var(--text-tertiary)" />}
                    <StatusChip status={d.status} />
                  </div>
                }
              />
            ))}
          </div>
        </div>

        <div className="card" style={{ flex: 1, minWidth: 260 }}>
          <div className="panel-head" style={{ borderBottom: "1px solid var(--border)" }}>
            <div className="disp" style={{ fontSize: 13, fontWeight: 600 }}>Recent activity</div>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 11 }}>
            {transfers.slice(0, 4).map((t) => (
              <div key={t.id} style={{ display: "flex", gap: 9, alignItems: "flex-start" }}>
                <div style={{ marginTop: 2, color: t.status === "failed" ? "var(--red)" : t.status === "completed" ? "var(--teal)" : "var(--blue)" }}>
                  {t.direction === "send" ? <ArrowUpRight size={13} /> : <ArrowDownLeft size={13} />}
                </div>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 12, fontWeight: 500 }}>{t.file}</div>
                  <div style={{ fontSize: 10.5, color: "var(--text-tertiary)" }}>
                    {t.direction === "send" ? `to ${t.target}` : `from ${t.target}`} · {t.status}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
