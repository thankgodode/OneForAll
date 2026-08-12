import React, { useState, useRef } from "react";
import { Upload, FileText, X, Check, Send } from "lucide-react";
import DeviceRow from "../components/DeviceRow";
import StatusChip from "../components/StatusChip";
import Toggle from "../components/Toggle";
import { fmtBytes } from "../utils/format";

// Send-files flow: pick files (drag-and-drop or file picker), pick one or
// more recipients, then hand both off to `onSend(files, recipientNames)`.
// All selection state is local; the parent only needs the final payload.
export default function SendFilesView({ devices, onSend, toast }) {
  const [files, setFiles] = useState([]);
  const [selected, setSelected] = useState([]);
  const [dragActive, setDragActive] = useState(false);
  const [confirmReq, setConfirmReq] = useState(true);
  const inputRef = useRef(null);
  const recipients = devices.filter((d) => !d.isSelf && d.status !== "offline");

  const addFiles = (list) => {
    const arr = Array.from(list).map((f) => ({ name: f.name, size: f.size }));
    setFiles((prev) => [...prev, ...arr]);
  };

  const toggleRecipient = (id) => {
    setSelected((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]));
  };

  const send = () => {
    if (!files.length || !selected.length) return;
    onSend(files, selected.map((id) => devices.find((d) => d.id === id).name));
    toast(`Sending ${files.length} file${files.length > 1 ? "s" : ""} to ${selected.length} recipient${selected.length > 1 ? "s" : ""}`);
    setFiles([]);
    setSelected([]);
  };

  return (
    <div className="fade-up" style={{ display: "flex", gap: 20, alignItems: "flex-start", flexWrap: "wrap" }}>
      <div className="card" style={{ flex: 1.3, minWidth: 340, display: "flex", flexDirection: "column", gap: 16 }}>
        <div className="disp" style={{ fontSize: 15, fontWeight: 600 }}>Select files</div>
        <div
          className={`dropzone ${dragActive ? "active" : ""}`}
          onDragOver={(e) => { e.preventDefault(); setDragActive(true); }}
          onDragLeave={() => setDragActive(false)}
          onDrop={(e) => { e.preventDefault(); setDragActive(false); addFiles(e.dataTransfer.files); }}
          onClick={() => inputRef.current.click()}
          style={{ cursor: "pointer" }}
        >
          <input ref={inputRef} type="file" multiple style={{ display: "none" }} onChange={(e) => addFiles(e.target.files)} />
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 8 }}>
            <div style={{ width: 40, height: 40, borderRadius: 10, background: "var(--teal-glow)", color: "var(--teal)", display: "flex", alignItems: "center", justifyContent: "center" }}>
              <Upload size={18} />
            </div>
            <div style={{ fontSize: 13, fontWeight: 600 }}>Drag files here, or click to browse</div>
            <div style={{ fontSize: 11.5, color: "var(--text-tertiary)" }}>Any file type · multiple files supported</div>
          </div>
        </div>

        {files.length > 0 && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {files.map((f, i) => (
              <div key={i} style={{ display: "flex", alignItems: "center", gap: 10, padding: "8px 10px", background: "var(--bg-elevated)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border)" }}>
                <FileText size={14} color="var(--text-tertiary)" />
                <div style={{ flex: 1, fontSize: 12.5 }}>{f.name}</div>
                <div className="mono" style={{ fontSize: 11, color: "var(--text-tertiary)" }}>{fmtBytes(f.size)}</div>
                <button className="btn-icon" style={{ border: "none", padding: 4 }} onClick={() => setFiles((prev) => prev.filter((_, idx) => idx !== i))}>
                  <X size={13} />
                </button>
              </div>
            ))}
          </div>
        )}

        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", paddingTop: 4, borderTop: "1px solid var(--border)" }}>
          <div style={{ fontSize: 12.5, color: "var(--text-secondary)" }}>Require confirmation before send</div>
          <Toggle on={confirmReq} onClick={() => setConfirmReq((v) => !v)} />
        </div>
      </div>

      <div className="card" style={{ flex: 1, minWidth: 300 }}>
        <div className="disp" style={{ fontSize: 15, fontWeight: 600, marginBottom: 4 }}>Choose recipients</div>
        <div style={{ fontSize: 12, color: "var(--text-tertiary)", marginBottom: 12 }}>{selected.length} selected</div>
        <div style={{ display: "flex", flexDirection: "column", gap: 2, marginBottom: 16 }}>
          {recipients.map((d) => (
            <DeviceRow
              key={d.id}
              device={d}
              size={32}
              selected={selected.includes(d.id)}
              onClick={() => toggleRecipient(d.id)}
              trailing={
                <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <StatusChip status={d.status} />
                  <div style={{
                    width: 18, height: 18, borderRadius: 5, border: `1.5px solid ${selected.includes(d.id) ? "var(--teal)" : "var(--border-strong)"}`,
                    background: selected.includes(d.id) ? "var(--teal)" : "transparent",
                    display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0,
                  }}>
                    {selected.includes(d.id) && <Check size={12} color="#06201B" />}
                  </div>
                </div>
              }
            />
          ))}
          {recipients.length === 0 && <div className="empty-state">No devices available right now</div>}
        </div>
        <button className="btn btn-primary" style={{ width: "100%", justifyContent: "center" }} disabled={!files.length || !selected.length} onClick={send}>
          <Send size={14} /> Send {files.length ? `${files.length} file${files.length > 1 ? "s" : ""}` : ""}
        </button>
      </div>
    </div>
  );
}
