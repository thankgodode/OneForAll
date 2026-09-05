import React, { useState } from "react";
import { Check, LogOut, Plus, Radio, Wifi, X } from "lucide-react";
import DeviceRow from "../components/DeviceRow";
import Toggle from "../components/Toggle";
import { toUiDevice } from "../utils/peer";

// Three distinct states based on network.role — "none" shows host/join
// tabs, "host" shows session management, "joined" shows a participant
// view with a Leave button. Unlike the earlier mock, discovered hosts
// come from real mDNS events and clicking one sends a real join request
// that may need the host's approval before it resolves.
export default function NetworkView({
  network, deviceName, discoveredHosts, pendingJoins, devices, accessControl,
  onStartHosting, onStopHosting, onJoin, onLeaveNetwork, onRespondToJoin, onKick,
  onSetAccessControl, toast, confirm,
}) {
  const [tab, setTab] = useState("host");
  const [sessionNameInput, setSessionNameInput] = useState(`${deviceName}'s Network`);
  const [manualAddr, setManualAddr] = useState("");
  const [requireApproval, setRequireApproval] = useState(accessControl.requireApproval);
  const [joinCode, setJoinCode] = useState(accessControl.joinCode);

  const participants = devices.filter((d) => !d.isSelf);
  // Filter out our own advertised service so hosting doesn't show up as
  // a joinable option in our own discovery list.
  const otherHosts = discoveredHosts.filter((h) => !h.fullname.startsWith(deviceName));

  const applyAccessControl = (nextRequireApproval, nextCode) => {
    setRequireApproval(nextRequireApproval);
    setJoinCode(nextCode);
    onSetAccessControl(nextRequireApproval, nextCode);
  };

  if (network.role === "host") {
    return (
      <div className="fade-up" style={{ display: "flex", gap: 14, flexWrap: "wrap" }}>
        <div className="card" style={{ flex: 1, minWidth: 320, display: "flex", flexDirection: "column", gap: 12 }}>
          <div>
            <div style={{ fontSize: 10.5, color: "var(--text-tertiary)", fontWeight: 600, textTransform: "uppercase", letterSpacing: ".03em" }}>Session name</div>
            <div className="disp" style={{ fontSize: 16, fontWeight: 600, marginTop: 3 }}>{network.sessionName}</div>
          </div>

          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", paddingTop: 4, borderTop: "1px solid var(--border)" }}>
            <div>
              <div style={{ fontSize: 12, fontWeight: 500 }}>Require approval to join</div>
              <div style={{ fontSize: 10.5, color: "var(--text-tertiary)" }}>Off = anyone nearby joins instantly</div>
            </div>
            <Toggle on={requireApproval} onClick={() => applyAccessControl(!requireApproval, joinCode)} />
          </div>

          <div>
            <div style={{ fontSize: 10.5, fontWeight: 600, color: "var(--text-tertiary)", marginBottom: 5, textTransform: "uppercase", letterSpacing: ".03em" }}>
              Optional join code
            </div>
            <div style={{ display: "flex", gap: 7 }}>
              <input
                type="text" className="mono" placeholder="Leave blank to require approval only"
                value={joinCode} onChange={(e) => setJoinCode(e.target.value)} style={{ flex: 1 }}
              />
              <button className="btn btn-ghost" onClick={() => applyAccessControl(requireApproval, joinCode)}>Set</button>
            </div>
          </div>

          <button className="btn btn-danger" onClick={() => confirm({
            title: "Stop hosting network?",
            body: "Everyone connected will be disconnected immediately.",
            confirmLabel: "Stop hosting",
            onConfirm: onStopHosting,
          })}>
            <LogOut size={13} /> Stop hosting
          </button>
        </div>

        <div className="card" style={{ flex: 1, minWidth: 320, display: "flex", flexDirection: "column", gap: 10 }}>
          {pendingJoins.length > 0 && (
            <div>
              <div className="panel-head"><div className="disp" style={{ fontSize: 13, fontWeight: 600 }}>Waiting for approval</div></div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {pendingJoins.map((peer) => (
                  <div key={peer.id} className="device-row">
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ fontSize: 12.5, fontWeight: 600 }}>{peer.display_name}</div>
                      <div className="mono" style={{ fontSize: 10.5, color: "var(--text-tertiary)" }}>{peer.device_name}</div>
                    </div>
                    <div style={{ display: "flex", gap: 5 }}>
                      <button className="btn-icon" onClick={() => onRespondToJoin(peer.id, true)}><Check size={13} /></button>
                      <button className="btn-icon" onClick={() => onRespondToJoin(peer.id, false)}><X size={13} /></button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          <div className="panel-head"><div className="disp" style={{ fontSize: 13, fontWeight: 600 }}>Connected participants</div><span className="mono" style={{ fontSize: 10.5, color: "var(--text-tertiary)" }}>{participants.length}</span></div>
          <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
            {participants.map((d) => (
              <DeviceRow key={d.id} device={d} size={28} trailing={
                <button className="btn-icon" onClick={() => confirm({
                  title: `Remove ${d.name}?`,
                  body: `${d.name} will be disconnected from this network session.`,
                  confirmLabel: "Remove",
                  onConfirm: () => onKick(d.id),
                })}>
                  <X size={12} />
                </button>
              } />
            ))}
            {participants.length === 0 && <div className="empty-state" style={{ padding: 20 }}>No one's joined yet</div>}
          </div>
        </div>
      </div>
    );
  }

  if (network.role === "joined") {
    return (
      <div className="fade-up" style={{ display: "flex", gap: 14, flexWrap: "wrap" }}>
        <div className="card" style={{ flex: 1, minWidth: 320, display: "flex", flexDirection: "column", gap: 12 }}>
          <div>
            <div style={{ fontSize: 10.5, color: "var(--text-tertiary)", fontWeight: 600, textTransform: "uppercase", letterSpacing: ".03em" }}>Connected to</div>
            <div className="disp" style={{ fontSize: 16, fontWeight: 600, marginTop: 3 }}>{network.sessionName}</div>
          </div>
          <button className="btn btn-danger" onClick={onLeaveNetwork}><LogOut size={13} /> Leave network</button>
        </div>

        <div className="card" style={{ flex: 1, minWidth: 320 }}>
          <div className="panel-head"><div className="disp" style={{ fontSize: 13, fontWeight: 600 }}>Connected participants</div><span className="mono" style={{ fontSize: 10.5, color: "var(--text-tertiary)" }}>{participants.length}</span></div>
          <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
            {participants.map((d) => (
              <DeviceRow
                key={d.id}
                device={d}
                size={28}
                trailing={d.isHost ? <span className="chip" style={{ background: "var(--teal-glow)", color: "var(--teal)" }}>Host</span> : undefined}
              />
            ))}
            {participants.length === 0 && <div className="empty-state" style={{ padding: 20 }}>Just you and the host so far</div>}
          </div>
        </div>
      </div>
    );
  }

  // network.role === "none" — pick host or join
  return (
    <div className="fade-up" style={{ display: "flex", gap: 14, flexWrap: "wrap" }}>
      <div className="card" style={{ flex: 1, minWidth: 320 }}>
        <div style={{ display: "flex", gap: 3, background: "var(--bg-elevated)", padding: 3, borderRadius: "var(--radius-sm)", marginBottom: 14, border: "1px solid var(--border)" }}>
          {["host", "join"].map((m) => (
            <button key={m} onClick={() => setTab(m)} className="btn" style={{
              flex: 1, justifyContent: "center", background: tab === m ? "var(--panel)" : "transparent",
              color: tab === m ? "var(--text-primary)" : "var(--text-tertiary)", border: "none",
            }}>
              {m === "host" ? "Host network" : "Join network"}
            </button>
          ))}
        </div>

        {tab === "host" ? (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div>
              <div style={{ fontSize: 10.5, fontWeight: 600, color: "var(--text-tertiary)", marginBottom: 6, textTransform: "uppercase", letterSpacing: ".03em" }}>Session name</div>
              <input type="text" value={sessionNameInput} onChange={(e) => setSessionNameInput(e.target.value)} style={{ width: "100%" }} />
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.5 }}>
              Nearby devices will be able to discover this session and request to join.
            </div>
            <button className="btn btn-primary" style={{ justifyContent: "center" }} onClick={() => onStartHosting(sessionNameInput)}>
              <Plus size={13} /> Start hosting
            </button>
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {network.joinStatus === "connecting" && (
              <div className="empty-state" style={{ padding: 16, background: "var(--bg-elevated)", borderRadius: "var(--radius-md)", border: "1px solid var(--border)" }}>
                <Radio size={16} /><div style={{ fontSize: 12 }}>Connecting…</div>
              </div>
            )}
            {network.joinStatus === "pending" && (
              <div className="empty-state" style={{ padding: 16, background: "var(--bg-elevated)", borderRadius: "var(--radius-md)", border: "1px solid var(--border)" }}>
                <Radio size={16} /><div style={{ fontSize: 12 }}>Waiting for the host to approve…</div>
              </div>
            )}

            {network.joinStatus === "idle" && (
              <>
                <div style={{ fontSize: 10.5, fontWeight: 600, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: ".03em" }}>Nearby hosts</div>
                <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
                  {otherHosts.map((h) => {
                    const label = h.fullname.split(".")[0];
                    return (
                      <div key={h.fullname} className="device-row" style={{ cursor: "pointer" }} onClick={() => onJoin(h.addr)}>
                        <div style={{ width: 30, height: 30, borderRadius: 7, background: "var(--teal-glow)", color: "var(--teal)", display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
                          <Wifi size={14} />
                        </div>
                        <div style={{ flex: 1, minWidth: 0 }}>
                          <div style={{ fontSize: 12.5, fontWeight: 600 }}>{label}</div>
                          <div className="mono" style={{ fontSize: 10.5, color: "var(--text-tertiary)" }}>{h.addr}</div>
                        </div>
                      </div>
                    );
                  })}
                  {otherHosts.length === 0 && <div className="empty-state" style={{ padding: 20 }}>Searching for hosts on your LAN…</div>}
                </div>

                <div style={{ fontSize: 10.5, color: "var(--text-tertiary)", fontWeight: 600, textTransform: "uppercase", letterSpacing: ".03em", marginTop: 6 }}>Or connect by address</div>
                <div style={{ display: "flex", gap: 7 }}>
                  <input type="text" placeholder="192.168.1.42:51820" value={manualAddr} onChange={(e) => setManualAddr(e.target.value)} style={{ flex: 1 }} className="mono" />
                  <button className="btn btn-primary" disabled={!manualAddr} onClick={() => onJoin(manualAddr)}>Join</button>
                </div>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
