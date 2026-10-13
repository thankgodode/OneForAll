import React from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Bell, ShieldCheck, Printer, HardDrive, FolderOpen } from "lucide-react";
import Toggle from "../components/Toggle";
import { initials } from "../utils/format";

const AVATAR_COLORS = ["#3FDCC4", "#5B9FE0", "#E8A63D", "#B383E8", "#E86B95"];

function SettingsRow({ label, desc, on, onClick }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "10px 0", borderBottom: "1px solid var(--border)" }}>
      <div>
        <div style={{ fontSize: 13, fontWeight: 500 }}>{label}</div>
        <div style={{ fontSize: 11.5, color: "var(--text-tertiary)" }}>{desc}</div>
      </div>
      <Toggle on={on} onClick={onClick} />
    </div>
  );
}

// Profile (local-only), notification prefs (local-only — no backend
// support yet), and the two settings that ARE real: confirm-incoming-files
// and download location, both wired to actual backend commands via
// `onSetConfirmFiles`/`onSetDownloadDir`. Printing prefs stay mock,
// matching PrintView, since there's no backend support for printing yet.
export default function SettingsView({ profile, setProfile, onSetDisplayName, onSetAvatarColor, onSetConfirmFiles, onSetConfirmPrinting, onSetDownloadDir, deviceName, myListenPort, toast }) {
  const patchGroup = (group, key) => setProfile((p) => ({ ...p, [group]: { ...p[group], [key]: !p[group][key] } }));

  console.log("PROFILE SETTINGS: ", profile);

  const pickDownloadDir = async () => {
    const dir = await open({ directory: true });
    if (dir) onSetDownloadDir(dir);
  };

  const deviceInfo = [
    ["Device name", deviceName || "—"],
    ["Listening port", myListenPort ?? "—"],
    ["App version", "1.4.2"],
  ];

  return (
    <div className="fade-up" style={{ display: "flex", gap: 14, flexWrap: "wrap" }}>
      <div className="card" style={{ flex: 1, minWidth: 320, display: "flex", flexDirection: "column", gap: 18 }}>
        <div className="disp" style={{ fontSize: 13, fontWeight: 600 }}>Profile</div>
        <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
          <div className="avatar" style={{ width: 52, height: 52, background: `${profile.avatarColor}22`, color: profile.avatarColor, border: `1px solid ${profile.avatarColor}44`, fontSize: 17 }}>
            {initials(profile.displayName || "Y")}
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            {AVATAR_COLORS.map((c) => (
              <button
                key={c}
                onClick={() => onSetAvatarColor(c)}
                style={{ width: 22, height: 22, borderRadius: "50%", background: c, border: profile.avatarColor === c ? "2px solid var(--text-primary)" : "2px solid transparent" }}
              />
            ))}
          </div>
        </div>

        <div>
          <div style={{ fontSize: 11.5, fontWeight: 600, color: "var(--text-tertiary)", marginBottom: 6, textTransform: "uppercase", letterSpacing: ".03em" }}>
            Display name
          </div>
          <div style={{ fontSize: 11, color: "var(--text-tertiary)", marginBottom: 6 }}>Shown to others when you join or host a network</div>
          <input type="text" value={profile.displayName} onChange={(e) => setProfile((p) => ({ ...p, displayName: e.target.value }))} style={{ width: "100%" }} />
        </div>

        <div>
          <div style={{ fontSize: 11.5, fontWeight: 600, color: "var(--text-tertiary)", marginBottom: 6, textTransform: "uppercase", letterSpacing: ".03em" }}>
            Download location
          </div>
          <div style={{ display: "flex", gap: 7 }}>
            <div className="mono" style={{ flex: 1, background: "var(--bg-elevated)", border: "1px solid var(--border)", borderRadius: "var(--radius-sm)", padding: "9px 11px", fontSize: 12, color: "var(--text-secondary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {profile.downloadPath || "Not set — using system default"}
            </div>
            <button className="btn btn-ghost" onClick={pickDownloadDir}><FolderOpen size={13} /></button>
          </div>
        </div>

        <button
          className="btn btn-primary"
          style={{ alignSelf: "flex-start" }}
          onClick={() => { onSetDisplayName(profile.displayName); toast("Profile saved"); }}
        >
          Save changes
        </button>
      </div>

      <div className="card" style={{ flex: 1, minWidth: 320, display: "flex", flexDirection: "column", gap: 4 }}>
        <div className="disp" style={{ fontSize: 13, fontWeight: 600, marginBottom: 4, display: "flex", alignItems: "center", gap: 8 }}>
          <Bell size={15} color="var(--text-tertiary)" /> Notifications
        </div>
        <SettingsRow label="Incoming file alerts" desc="Notify when someone sends you a file" on={profile.notifications.incoming} onClick={() => patchGroup("notifications", "incoming")} />
        <SettingsRow label="Transfer complete" desc="Notify when a transfer finishes" on={profile.notifications.complete} onClick={() => patchGroup("notifications", "complete")} />
        <SettingsRow label="Print job updates" desc="Notify on print job status changes" on={profile.notifications.print} onClick={() => patchGroup("notifications", "print")} />
        <SettingsRow label="Sound alerts" desc="Play notification sounds when the app is in the background" on={profile.notifications.sound} onClick={() => patchGroup("notifications", "sound")} />

        <div className="disp" style={{ fontSize: 13, fontWeight: 600, margin: "18px 0 4px", display: "flex", alignItems: "center", gap: 8 }}>
          <ShieldCheck size={15} color="var(--text-tertiary)" /> Confirmations
        </div>
        <SettingsRow
          label="Confirm incoming files"
          desc="Require approval before accepting files"
          on={profile.confirmations.files}
          onClick={() => onSetConfirmFiles(!profile.confirmations.files)}
        />
        <SettingsRow 
          label="Confirm incoming print jobs" 
          desc="Require approval before printing" on={profile.confirmations.prints} 
          onClick={() => onSetConfirmPrinting(!profile.confirmations.prints)} 
        />

        <div className="disp" style={{ fontSize: 13, fontWeight: 600, margin: "18px 0 4px", display: "flex", alignItems: "center", gap: 8 }}>
          <Printer size={15} color="var(--text-tertiary)" /> Printing
        </div>
        {/* <SettingsRow label="Auto-accept trusted devices" desc="Not implemented yet — printing is UI-only for now" on={profile.printPrefs.autoAccept} onClick={() => patchGroup("printPrefs", "autoAccept")} />
        <SettingsRow label="Print in background" desc="Not implemented yet — printing is UI-only for now" on={profile.printPrefs.background} onClick={() => patchGroup("printPrefs", "background")} /> */}
      </div>

      <div className="card" style={{ flex: 1, minWidth: 260 }}>
        <div className="disp" style={{ fontSize: 13, fontWeight: 600, marginBottom: 12, display: "flex", alignItems: "center", gap: 8 }}>
          <HardDrive size={15} color="var(--text-tertiary)" /> This device
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          {deviceInfo.map(([k, v]) => (
            <div key={k} style={{ display: "flex", justifyContent: "space-between" }}>
              <span style={{ fontSize: 12.5, color: "var(--text-tertiary)" }}>{k}</span>
              <span className="mono" style={{ fontSize: 12.5 }}>{v}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
