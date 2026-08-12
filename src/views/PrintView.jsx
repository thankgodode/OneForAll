import React, { useState, useRef } from "react";
import { Printer, FileText, X, FolderOpen, Check, Loader2 } from "lucide-react";
import DeviceRow from "../components/DeviceRow";
import { JOB_STATUS_META } from "../data/mockData";

// Print flow: choose a document, choose a device with a printer attached,
// submit a job. Bottom half lists print jobs with pending/printing/done/
// rejected status and inline accept/reject controls for pending jobs.
export default function PrintView({ devices, printJobs, onPrint, onJobAction, toast }) {
  const [doc, setDoc] = useState(null);
  const [target, setTarget] = useState(null);
  const inputRef = useRef(null);
  const printers = devices.filter((d) => d.hasPrinter && !d.isSelf && d.status !== "offline");

  const submit = () => {
    if (!doc || !target) return;
    const t = devices.find((d) => d.id === target);
    onPrint(doc, t);
    toast(`Print job sent to ${t.name} (${t.printerName})`);
    setDoc(null);
    setTarget(null);
  };

  return (
    <div className="fade-up" style={{ display: "flex", gap: 20, flexWrap: "wrap" }}>
      <div className="card" style={{ flex: 1, minWidth: 320, display: "flex", flexDirection: "column", gap: 16 }}>
        <div className="disp" style={{ fontSize: 15, fontWeight: 600 }}>New print job</div>

        <div>
          <div style={{ fontSize: 11.5, fontWeight: 600, color: "var(--text-tertiary)", marginBottom: 8, textTransform: "uppercase", letterSpacing: ".03em" }}>Document</div>
          <input ref={inputRef} type="file" style={{ display: "none" }} onChange={(e) => e.target.files[0] && setDoc({ name: e.target.files[0].name, size: e.target.files[0].size })} />
          {doc ? (
            <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "10px 12px", background: "var(--bg-elevated)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border)" }}>
              <FileText size={15} color="var(--amber)" />
              <div style={{ flex: 1, fontSize: 12.5 }}>{doc.name}</div>
              <button className="btn-icon" style={{ border: "none", padding: 4 }} onClick={() => setDoc(null)}><X size={13} /></button>
            </div>
          ) : (
            <button className="btn btn-ghost" style={{ width: "100%", justifyContent: "center" }} onClick={() => inputRef.current.click()}>
              <FolderOpen size={14} /> Choose a document
            </button>
          )}
        </div>

        <div>
          <div style={{ fontSize: 11.5, fontWeight: 600, color: "var(--text-tertiary)", marginBottom: 8, textTransform: "uppercase", letterSpacing: ".03em" }}>Destination printer</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            {printers.map((d) => (
              <DeviceRow
                key={d.id}
                device={d}
                size={32}
                selected={target === d.id}
                onClick={() => setTarget(d.id)}
                subtitle={d.printerName}
                trailing={<Printer size={14} color="var(--text-tertiary)" />}
              />
            ))}
            {printers.length === 0 && <div className="empty-state" style={{ padding: 24 }}>No printers available on the network</div>}
          </div>
        </div>

        <button className="btn btn-primary" style={{ width: "100%", justifyContent: "center" }} disabled={!doc || !target} onClick={submit}>
          <Printer size={14} /> Send to print
        </button>
      </div>

      <div className="card" style={{ flex: 1, minWidth: 320 }}>
        <div className="disp" style={{ fontSize: 15, fontWeight: 600, marginBottom: 12 }}>Print jobs</div>
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          {printJobs.map((p) => {
            const m = JOB_STATUS_META[p.status];
            return (
              <div key={p.id} style={{ display: "flex", alignItems: "center", gap: 12, padding: 12, background: "var(--bg-elevated)", borderRadius: "var(--radius-md)", border: "1px solid var(--border)" }}>
                <div style={{ width: 30, height: 30, borderRadius: 8, background: `${m.color}1A`, color: m.color, display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
                  {p.status === "printing" ? <Loader2 size={14} /> : <Printer size={14} />}
                </div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 12.5, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{p.file}</div>
                  <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>{p.target} · {p.pages} page{p.pages > 1 ? "s" : ""}</div>
                </div>
                <span className="chip" style={{ background: `${m.color}1A`, color: m.color }}>{m.label}</span>
                {p.status === "pending" && (
                  <div style={{ display: "flex", gap: 4 }}>
                    <button className="btn-icon" onClick={() => onJobAction(p.id, "completed")}><Check size={13} /></button>
                    <button className="btn-icon" onClick={() => onJobAction(p.id, "rejected")}><X size={13} /></button>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
