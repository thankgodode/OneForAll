import React, { useEffect, useState } from "react";
import { Copy, Link, Link2Icon, List, LogOut, Plus, Radio, X } from "lucide-react";
import DeviceRow from "../components/DeviceRow";
import { invoke } from '@tauri-apps/api/core';
import { listen } from "@tauri-apps/api/event";

// Host/join management plus the live participants list.
// `confirm` is App's `setModal` — passed down so this view can trigger the
// shared ConfirmModal without owning any modal state itself.
export default function NetworkView({ network, setNetwork, devices, toast, confirm }) {
  const [mode, setMode] = useState(network.mode);
  const [services, setServices] = useState([]);
  const [browsing, setBrowsing] = useState(false);
  const [joinCode, setJoinCode] = useState("");
  const copyCode = () => toast("Session code copied to clipboard");
  const participants = devices.filter((d) => !d.isSelf && d.status !== "offline");

  useEffect(() => {
    let unlistenResolved;
    let unlistenRemoved;

    const setupListeners = async () => {
      unlistenResolved = await listen(
        "mdns-service-resolved",
        (event) => {
          setServices((current) => {
            const updated = new Map(current);

            updated.set(event.payload[0], {
              name: event.payload[0],
              address: event.payload[1],
            });

            return updated;
        });
        }
      );

      unlistenRemoved = await listen(
        "mdns-service-removed",
        (event) => {
          setServices((current) => {
            const updated = new Map(current);

            updated.delete(event.payload);

            return updated;
          });
        }
      );
      
      console.log("SERVICES: ", services);
    };

    setupListeners();


    return () => {
      unlistenResolved?.();
      unlistenRemoved?.();
    };


  }, [browsing]);

  return (
    <div className="fade-up" style={{ display: "flex", gap: 20, flexWrap: "wrap" }}>
      <div className="card" style={{ flex: 1, minWidth: 320 }}>
        <div style={{ display: "flex", gap: 4, background: "var(--bg-elevated)", padding: 4, borderRadius: "var(--radius-sm)", marginBottom: 18, border: "1px solid var(--border)" }}>
          {["host", "join"].map((m) => (
            <button
              key={m}
              onClick={() => setMode(m)}
              className="btn"
              style={{ flex: 1, justifyContent: "center", background: mode === m ? "var(--panel)" : "transparent", color: mode === m ? "var(--text-primary)" : "var(--text-tertiary)", border: "none" }}
            >
              {m === "host" ? "Host network" : "Join network"}
            </button>
          ))}
        </div>

        {mode === "host" ? (
          network.connected ? (
            <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
              <div>
                <div style={{ fontSize: 11.5, color: "var(--text-tertiary)", fontWeight: 600, textTransform: "uppercase", letterSpacing: ".03em" }}>Session name</div>
                <div className="disp" style={{ fontSize: 18, fontWeight: 600, marginTop: 4 }}>{network.sessionName}</div>
              </div>
              <div>
                <div style={{ fontSize: 11.5, color: "var(--text-tertiary)", fontWeight: 600, textTransform: "uppercase", letterSpacing: ".03em", marginBottom: 6 }}>Connection code</div>
                <div style={{ display: "flex", gap: 8 }}>
                  <div className="mono" style={{ flex: 1, background: "var(--bg-elevated)", border: "1px solid var(--border)", borderRadius: "var(--radius-sm)", padding: "10px 12px", fontSize: 15, letterSpacing: "0.06em", color: "var(--teal)" }}>
                    {network.sessionCode}
                  </div>
                  <button className="btn-icon" onClick={copyCode}><Copy size={14} /></button>
                </div>
              </div>
              <button
                className="btn btn-danger"
                onClick={() => confirm({
                  title: "Stop hosting network?",
                  body: "Everyone connected will be disconnected immediately.",
                  confirmLabel: "Stop hosting",
                  onConfirm: () => { setNetwork((n) => ({ ...n, connected: false })); toast("Stopped hosting the network"); },
                })}
              >
                <LogOut size={14} /> Stop hosting
              </button>
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <div style={{ fontSize: 12.5, color: "var(--text-secondary)", lineHeight: 1.5 }}>Start a session so nearby devices can discover and connect to you.</div>
              <button className="btn btn-primary" style={{ justifyContent: "center" }} onClick={async() => { 
                await invoke("start_hosting",{sessionName:"first-session", instanceName:"THANKGOD-PC"});
                
                setNetwork((n) => ({ ...n, connected: true, mode: "host" 
              })); toast("Network session started"); }}>
                <Plus size={14} /> Start hosting
              </button>
            </div>
          )
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          {!browsing && <button className="btn btn-primary" 
            style={{ justifyContent: "center" }} 
            onClick={async() => { 
              await invoke("browse_service");
              setBrowsing(true)
            }}
          >
            <Link2Icon size={14} /> Join a network
          </button>
          }
          {
            browsing && services.size < 1 && 
              <div className="empty-state" style={{ padding: 20, background: "var(--bg-elevated)", borderRadius: "var(--radius-md)", border: "1px solid var(--border)" }}>
                <Radio size={18} />
                <div style={{ fontSize: 12.5 }}>Searching for hosts on your LAN…</div>
              </div>
          }
          {services.size > 0 && (
            <div>
              {[...services.values()].map((service) => {
                return(
                  <div key={service.name} style={{ display: "flex", flexDirection: "column", gap: 2, padding: 8, background: "var(--bg-elevated)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border)", marginBottom: 6, cursor:"pointer"}}>
                    <label>
                      {service.name}
                    </label>
                  </div>
                )
              })}
            </div>
          )}
          {browsing &&(
            <button
              className="btn btn-danger"
              onClick={async()=>{
                await invoke("stop_browsing")
                setServices(new Map())
                setBrowsing(false)
              }}
            >
              Stop browsing
            </button>
          )}
            {/* <div style={{ fontSize: 11.5, color: "var(--text-tertiary)", fontWeight: 600, textTransform: "uppercase", letterSpacing: ".03em" }}>Or enter a code manually</div> */}
            {/* <div style={{ display: "flex", gap: 8 }}>
              <input type="text" placeholder="e.g. FIR-482-QLT" value={joinCode} onChange={(e) => setJoinCode(e.target.value)} style={{ flex: 1 }} className="mono" />
              <button className="btn btn-primary" disabled={!joinCode} onClick={() => toast("Connecting to network…")}>Join</button>
            </div> */}
          </div>
        )}
      </div>

      <div className="card" style={{ flex: 1, minWidth: 320 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
          <div className="disp" style={{ fontSize: 15, fontWeight: 600 }}>Connected participants</div>
          <span className="mono" style={{ fontSize: 11, color: "var(--text-tertiary)" }}>{participants.length}</span>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
          {participants.map((d) => (
            <DeviceRow
              key={d.id}
              device={d}
              size={32}
              trailing={
                network.mode === "host" && network.connected ? (
                  <button
                    className="btn-icon"
                    onClick={() => confirm({
                      title: `Remove ${d.name}?`,
                      body: `${d.name} will be disconnected from this network session.`,
                      confirmLabel: "Remove",
                      onConfirm: () => toast(`${d.name} removed from the network`),
                    })}
                  >
                    <X size={13} />
                  </button>
                ) : undefined
              }
            />
          ))}
        </div>
      </div>
    </div>
  );
}
