import React, { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { stat } from "@tauri-apps/plugin-fs";
import { Printer, FileText, X, FolderOpen, Loader2 } from "lucide-react";
import { JOB_STATUS_META } from "../data/mockData";
import { fmtBytes } from "../utils/format";

// Print flow: select a local document and a printer advertised by a peer.
export default function PrintView({ devices, printers, printJobs, onPrint, toast }) {
  const [doc, setDoc] = useState(null);
  const [target, setTarget] = useState(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const availablePrinters = printers.filter((printer) => {
    const device = devices.find((item) => item.id === printer.peerId);
    return device && !device.isSelf && device.status !== "offline";
  });

  const chooseDocument = async () => {
    try {
      const path = await open({ multiple: false });
      if (!path) return;
      const info = await stat(path);
      if (!info.isFile) {
        toast("Choose a regular file to print", true);
        return;
      }
      setDoc({
        path,
        name: path.split(/[\\/]/).pop(),
        size: info.size,
      });
    } catch (error) {
      toast(`Couldn't open the selected document: ${error}`, true);
    }
  };

  const submit = async () => {
    if (!doc || !target || isSubmitting) return;
    setIsSubmitting(true);
    try {
      await onPrint(doc, target);
      setDoc(null);
      setTarget(null);
    } catch {
      // The parent reports backend errors with the shared toast.
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fade-up" style={{ display: "flex", gap: 14, flexWrap: "wrap" }}>
      <div className="card" style={{ flex: 1, minWidth: 320, display: "flex", flexDirection: "column", gap: 12 }}>
        <div className="disp" style={{ fontSize: 13, fontWeight: 600 }}>New print job</div>

        <div>
          <div style={{ fontSize: 11.5, fontWeight: 600, color: "var(--text-tertiary)", marginBottom: 8, textTransform: "uppercase", letterSpacing: ".03em" }}>Document</div>
          {doc ? (
            <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "10px 12px", background: "var(--bg-elevated)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border)" }}>
              <FileText size={15} color="var(--amber)" />
              <div style={{ flex: 1, minWidth: 0, fontSize: 12.5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{doc.name} · {fmtBytes(doc.size)}</div>
              <button className="btn-icon" style={{ border: "none", padding: 4 }} onClick={() => setDoc(null)}><X size={13} /></button>
            </div>
          ) : (
            <button className="btn btn-ghost" style={{ width: "100%", justifyContent: "center" }} onClick={chooseDocument}>
              <FolderOpen size={14} /> Choose a document
            </button>
          )}
        </div>

        <div>
          <div style={{ fontSize: 11.5, fontWeight: 600, color: "var(--text-tertiary)", marginBottom: 8, textTransform: "uppercase", letterSpacing: ".03em" }}>Destination printer</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            {availablePrinters.map((printer) => {
              const device = devices.find((item) => item.id === printer.peerId);
              return (
                <button
                  key={`${printer.peerId}:${printer.printerId}`}
                  className="btn btn-ghost"
                  style={{
                    width: "100%",
                    justifyContent: "space-between",
                    borderColor: target?.printerId === printer.printerId && target?.peerId === printer.peerId ? "var(--teal)" : "var(--border)",
                  }}
                  onClick={() => setTarget(printer)}
                >
                  <span style={{ display: "flex", flexDirection: "column", alignItems: "flex-start", gap: 2 }}>
                    <span>{printer.printerName}{printer.isDefault ? " · Default" : ""}</span>
                    <span style={{ fontSize: 11, color: "var(--text-tertiary)" }}>{device.name}</span>
                  </span>
                  <Printer size={14} color="var(--text-tertiary)" />
                </button>
              );
            })}
            {availablePrinters.length === 0 && <div className="empty-state" style={{ padding: 24 }}>No printers available on the network</div>}
          </div>
        </div>

        <button className="btn btn-primary" style={{ width: "100%", justifyContent: "center" }} disabled={!doc || !target || isSubmitting} onClick={submit}>
          <Printer size={14} /> Send to print
        </button>
      </div>

      <div className="card" style={{ flex: 1, minWidth: 320 }}>
        <div className="disp" style={{ fontSize: 13, fontWeight: 600, marginBottom: 12 }}>Print jobs</div>
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          {printJobs.map((p) => {
            const m = JOB_STATUS_META[p.status];
            return (
              <div key={p.id} style={{ display: "flex", alignItems: "center", gap: 12, padding: 12, background: "var(--bg-elevated)", borderRadius: "var(--radius-md)", border: "1px solid var(--border)" }}>
                <div style={{ width: 30, height: 30, borderRadius: 8, background: `${m.color}1A`, color: m.color, display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
                  {p.status === "printing" ? <Loader2 size={14} /> : <Printer size={14} />}
                </div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 12.5, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{p.filename}</div>
                  <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>{p.direction === "send" ? `To ${p.peerName}` : `From ${p.peerName}`} · {p.printerName}</div>
                </div>
                <span className="chip" title={p.error || undefined} style={{ background: `${m.color}1A`, color: m.color }}>{m.label}</span>
              </div>
            );
          })}
          {printJobs.length === 0 && <div className="empty-state" style={{ padding: 24 }}>No print jobs in this session</div>}
        </div>
      </div>
    </div>
  );
}
