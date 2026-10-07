import React, { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { stat } from "@tauri-apps/plugin-fs";
import { Printer, FileText, X, FolderOpen, Loader2 } from "lucide-react";
import DeviceRow from "../components/DeviceRow";
import { JOB_STATUS_META } from "../data/mockData";
import { fmtBytes } from "../utils/format";

export default function PrintView({ devices, printJobs, onPrint, toast }) {
  const [doc, setDoc] = useState(null);
  const [target, setTarget] = useState(null);
  const [copies, setCopies] = useState("1");
  const printers = devices
    .filter((device) => device.hasPrinter && !device.isSelf && device.status !== "offline")
    .flatMap((device) => device.printerNames.map((printerName) => ({ device, printerName })));

  const chooseDocument = async () => {
    try {
      const path = await open({ multiple: false });
      if (!path || Array.isArray(path)) return;
      const info = await stat(path);
      setDoc({ path, name: path.split(/[\\/]/).pop(), size: info.size });
    } catch (error) {
      toast(`Couldn't open that document: ${error}`, true);
    }
  };

  const submit = async () => {
    if (!doc || !target || !validCopies) return;
    const printer = printers.find(({ device, printerName }) => `${device.id}:${printerName}` === target);
    if (!printer) return;
    try {
      await onPrint(doc, printer.device, printer.printerName, Number(copies));
      setDoc(null);
      setTarget(null);
    } catch (error) {
      toast(`Couldn't send print job: ${error}`, true);
    }
  };
  const validCopies = Number.isSafeInteger(Number(copies)) && Number(copies) > 0 && Number(copies) <= 0xFFFFFFFF;

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
              <button className="btn-icon" style={{ border: "none", padding: 4 }} onClick={() => setDoc(null)} aria-label="Remove document"><X size={13} /></button>
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
            {printers.map(({ device, printerName }) => {
              const key = `${device.id}:${printerName}`;
              return (
                <DeviceRow
                  key={key}
                  device={device}
                  size={32}
                  selected={target === key}
                  onClick={() => setTarget(key)}
                  subtitle={printerName}
                  trailing={<Printer size={14} color="var(--text-tertiary)" />}
                />
              );
            })}
            {printers.length === 0 && <div className="empty-state" style={{ padding: 24 }}>No printers are available on connected devices</div>}
          </div>
        </div>

        <label style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12, fontSize: 12, color: "var(--text-secondary)" }}>
          Number of copies
          <input
            type="number"
            min="1"
            step="1"
            value={copies}
            onChange={(event) => setCopies(event.target.value)}
            aria-label="Number of copies"
            style={{ width: 88, padding: "7px 9px", color: "var(--text-primary)", background: "var(--bg-elevated)", border: "1px solid var(--border)", borderRadius: "var(--radius-sm)" }}
          />
        </label>
        {!validCopies && <div style={{ fontSize: 11, color: "var(--red)" }}>Enter a whole number greater than zero.</div>}

        <button className="btn btn-primary" style={{ width: "100%", justifyContent: "center" }} disabled={!doc || !target || !validCopies} onClick={submit}>
          <Printer size={14} /> Send print job
        </button>
        <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>The receiving device will ask before sending the document to its printer.</div>
      </div>

      <div className="card" style={{ flex: 1, minWidth: 320 }}>
        <div className="disp" style={{ fontSize: 13, fontWeight: 600, marginBottom: 12 }}>Print jobs</div>
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          {printJobs.map((job) => {
            const meta = JOB_STATUS_META[job.status] ?? JOB_STATUS_META.failed;
            return (
              <div key={job.id} style={{ display: "flex", alignItems: "center", gap: 12, padding: 12, background: "var(--bg-elevated)", borderRadius: "var(--radius-md)", border: "1px solid var(--border)" }}>
                <div style={{ width: 30, height: 30, borderRadius: 8, background: `${meta.color}1A`, color: meta.color, display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
                  {["sending", "receiving", "printing"].includes(job.status) ? <Loader2 size={14} /> : <Printer size={14} />}
                </div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 12.5, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{job.file}</div>
                  <div style={{ fontSize: 11, color: job.error ? "var(--red)" : "var(--text-tertiary)" }}>
                    {job.error || `${job.target} · ${job.printer} · ${job.copies} ${job.copies === 1 ? "copy" : "copies"}`}
                  </div>
                </div>
                <span className="chip" style={{ background: `${meta.color}1A`, color: meta.color }}>{meta.label}</span>
              </div>
            );
          })}
          {printJobs.length === 0 && <div className="empty-state" style={{ padding: 24 }}>No print jobs yet</div>}
        </div>
      </div>
    </div>
  );
}
