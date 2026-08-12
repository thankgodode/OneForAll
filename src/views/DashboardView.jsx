import React from "react";
import { Radio, ArrowUpRight, ArrowDownLeft, Printer, Wifi, ChevronRight } from "lucide-react";
import DeviceRow from "../components/DeviceRow";
import StatusChip from "../components/StatusChip";
import MeshView from "../components/MeshView";

function StatCard({ label, value, icon, color }) {
  return (
    <div className="card" style={{ flex: 1, minWidth: 140 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
        <div>
          <div style={{ fontSize: 11, color: "var(--text-tertiary)", fontWeight: 600, letterSpacing: ".03em", textTransform: "uppercase" }}>{label}</div>
          <div className="disp" style={{ fontSize: 28, fontWeight: 700, marginTop: 6 }}>{value}</div>
        </div>
        <div style={{ width: 34, height: 34, borderRadius: 9, background: `${color}1A`, color, display: "flex", alignItems: "center", justifyContent: "center" }}>{icon}</div>
      </div>
    </div>
  );
}

// Landing view: at-a-glance stats, the mesh diagram, the device list, and
// a rollup of recent transfer activity. Reads devices/transfers/printJobs
// as plain props — no local data fetching happens here.
export default function DashboardView({ devices, transfers, printJobs, network, setView }) {
  const online = devices.filter((d) => !d.isSelf && d.status !== "offline").length;
  const active = transfers.filter((t) => t.status === "active").length;
  const jobs = printJobs.filter((p) => p.status === "printing" || p.status === "pending").length;

  return (
    <div className="fade-up" style={{ display: "flex", flexDirection: "column", gap: 20 }}>
      <div style={{ display: "flex", gap: 14, flexWrap: "wrap" }}>
        <StatCard label="Devices online" value={online} icon={<Radio size={16} />} color="var(--teal)" />
        <StatCard label="Active transfers" value={active} icon={<ArrowUpRight size={16} />} color="var(--blue)" />
        <StatCard label="Print jobs" value={jobs} icon={<Printer size={16} />} color="var(--amber)" />
        <StatCard label="Network" value={network.connected ? "Connected" : "Offline"} icon={<Wifi size={16} />} color={network.connected ? "var(--teal)" : "var(--gray)"} />
      </div>

      <div className="card">
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
          <div>
            <div className="disp" style={{ fontSize: 15, fontWeight: 600 }}>Network topology</div>
            <div style={{ fontSize: 12, color: "var(--text-tertiary)" }}>Live view of who's reachable on {network.sessionName}</div>
          </div>
          <button className="btn btn-ghost" onClick={() => setView("network")}>Manage <ChevronRight size={14} /></button>
        </div>
        <MeshView devices={devices} />
      </div>

      <div style={{ display: "flex", gap: 20, flexWrap: "wrap" }}>
        <div className="card" style={{ flex: 2, minWidth: 320 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
            <div className="disp" style={{ fontSize: 15, fontWeight: 600 }}>Connected devices</div>
            <button className="btn btn-ghost" onClick={() => setView("send")}>Send files <ChevronRight size={14} /></button>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            {devices.filter((d) => !d.isSelf).map((d) => (
              <DeviceRow
                key={d.id}
                device={d}
                trailing={
                  <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                    {d.hasPrinter && <Printer size={14} color="var(--text-tertiary)" />}
                    <StatusChip status={d.status} />
                  </div>
                }
              />
            ))}
          </div>
        </div>

        <div className="card" style={{ flex: 1, minWidth: 260 }}>
          <div className="disp" style={{ fontSize: 15, fontWeight: 600, marginBottom: 12 }}>Recent activity</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
            {transfers.slice(0, 4).map((t) => (
              <div key={t.id} style={{ display: "flex", gap: 10, alignItems: "flex-start" }}>
                <div style={{ marginTop: 2, color: t.status === "failed" ? "var(--red)" : t.status === "completed" ? "var(--teal)" : "var(--blue)" }}>
                  {t.direction === "send" ? <ArrowUpRight size={14} /> : <ArrowDownLeft size={14} />}
                </div>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 12.5, fontWeight: 500 }}>{t.file}</div>
                  <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>
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
