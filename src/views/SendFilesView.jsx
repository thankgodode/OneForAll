import React, { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { stat } from "@tauri-apps/plugin-fs";
import { Upload, FileText, X, Check, Send } from "lucide-react";
import DeviceRow from "../components/DeviceRow";
import StatusChip from "../components/StatusChip";
import { fmtBytes } from "../utils/format";

// Send-files flow: pick files, pick one or more recipients, then hand
// real filesystem paths off to `onSend(filePaths, targetPeerIds)`.
//
// Uses Tauri's native file picker rather than an HTML <input type="file">
// — a browser file input only ever exposes a File object (name + size +
// in-memory bytes), never a real path on disk, and `send_file_to_peers`
// needs an actual path it can hand to `tokio::fs::File::open` on the
// Rust side. Native OS drag-and-drop of real files onto the window would
// need Tauri's own window-level drag-drop event instead of the browser's
// — not wired here, so dropping is a no-op with a toast pointing at the
// button instead of silently doing nothing.
export default function SendFilesView({ devices, onSend, toast }) {
  const [files, setFiles] = useState([]); // [{ path, name, size }]
  const [selected, setSelected] = useState([]);
  const [dragActive, setDragActive] = useState(false);
  const recipients = devices.filter((d) => !d.isSelf && d.status !== "offline");

  const pickFiles = async () => {
    const picked = await open({ multiple: true });
    if (!picked) return;
    const paths = Array.isArray(picked) ? picked : [picked];

    const withMeta = await Promise.all(paths.map(async (path) => {
      let size = 0;
      try {
        const info = await stat(path);
        size = info.size;
      } catch {
        // metadata read failed — still let the file through, just show 0 B
      }
      const name = path.split(/[\\/]/).pop();
      return { path, name, size };
    }));

    setFiles((prev) => [...prev, ...withMeta]);
  };

  const toggleRecipient = (id) => {
    setSelected((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]));
  };

  const send = () => {
    if (!files.length || !selected.length) return;
    onSend(files.map((f) => f.path), selected);
    toast(`Sending ${files.length} file${files.length > 1 ? "s" : ""} to ${selected.length} recipient${selected.length > 1 ? "s" : ""}`);
    setFiles([]);
    setSelected([]);
  };

  return (
    <div className="fade-up" style={{ display: "flex", gap: 14, alignItems: "flex-start", flexWrap: "wrap" }}>
      <div className="card" style={{ flex: 1.3, minWidth: 340, display: "flex", flexDirection: "column", gap: 12 }}>
        <div className="disp" style={{ fontSize: 13, fontWeight: 600 }}>Select files</div>
        <div
          className={`dropzone ${dragActive ? "active" : ""}`}
          onDragOver={(e) => { e.preventDefault(); setDragActive(true); }}
          onDragLeave={() => setDragActive(false)}
          onDrop={(e) => { e.preventDefault(); setDragActive(false); toast("Drag-and-drop isn't wired up yet — click to browse instead"); }}
          onClick={pickFiles}
          style={{ cursor: "pointer" }}
        >
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 8 }}>
            <div style={{ width: 40, height: 40, borderRadius: 10, background: "var(--teal-glow)", color: "var(--teal)", display: "flex", alignItems: "center", justifyContent: "center" }}>
              <Upload size={18} />
            </div>
            <div style={{ fontSize: 13, fontWeight: 600 }}>Click to browse</div>
            <div style={{ fontSize: 11.5, color: "var(--text-tertiary)" }}>Any file type · multiple files supported</div>
          </div>
        </div>

        {files.length > 0 && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {files.map((f, i) => (
              <div key={f.path} style={{ display: "flex", alignItems: "center", gap: 10, padding: "8px 10px", background: "var(--bg-elevated)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border)" }}>
                <FileText size={14} color="var(--text-tertiary)" />
                <div style={{ flex: 1, fontSize: 12.5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{f.name}</div>
                <div className="mono" style={{ fontSize: 11, color: "var(--text-tertiary)" }}>{fmtBytes(f.size)}</div>
                <button className="btn-icon" style={{ border: "none", padding: 4 }} onClick={() => setFiles((prev) => prev.filter((_, idx) => idx !== i))}>
                  <X size={13} />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="card" style={{ flex: 1, minWidth: 300 }}>
        <div className="disp" style={{ fontSize: 13, fontWeight: 600, marginBottom: 4 }}>Choose recipients</div>
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
          {recipients.length === 0 && <div className="empty-state">No devices available right now — host or join a network first</div>}
        </div>
        <button className="btn btn-primary" style={{ width: "100%", justifyContent: "center" }} disabled={!files.length || !selected.length} onClick={send}>
          <Send size={14} /> Send {files.length ? `${files.length} file${files.length > 1 ? "s" : ""}` : ""}
        </button>
      </div>
    </div>
  );
}
