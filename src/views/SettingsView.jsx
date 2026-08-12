import React from "react";
import { Bell, ShieldCheck, Printer, HardDrive } from "lucide-react";
import Toggle from "../components/Toggle";
import { initials } from "../utils/format";

const AVATAR_COLORS = ["#3FDCC4", "#5B9FE0", "#E8A63D", "#B383E8", "#E86B95"];
const DEVICE_INFO = [
  ["Device name", "Ada-Desktop-01"],
  ["Local IP", "192.168.1.42"],
  ["Network", "Home-Studio-5G"],
  ["App version", "1.4.2"],
];

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

// Profile, notification/confirmation toggles, printing prefs, and read-only
// device info. `profile`/`setProfile` come straight from the App's state —
// this view never mutates anything it doesn't own.
export default function SettingsView({ profile, setProfile, toast }) {
  const patchGroup = (group, key) => setProfile((p) => ({ ...p, [group]: { ...p[group], [key]: !p[group][key] } }));

  return (
    <div className="fade-up" style={{ display: "flex", gap: 20, flexWrap: "wrap" }}>
      <div className="card" style={{ flex: 1, minWidth: 320, display: "flex", flexDirection: "column", gap: 18 }}>
        <div className="disp" style={{ fontSize: 15, fontWeight: 600 }}>Profile</div>
        <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
          <div className="avatar" style={{ width: 52, height: 52, background: `${profile.avatarColor}22`, color: profile.avatarColor, border: `1px solid ${profile.avatarColor}44`, fontSize: 17 }}>
            {initials(profile.displayName || "Y")}
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            {AVATAR_COLORS.map((c) => (
              <button
                key={c}
                onClick={() => setProfile((p) => ({ ...p, avatarColor: c }))}
                style={{ width: 22, height: 22, borderRadius: "50%", background: c, border: profile.avatarColor === c ? "2px solid var(--text-primary)" : "2px solid transparent" }}
              />
            ))}
          </div>
        </div>

        <div>
          <div style={{ fontSize: 11.5, fontWeight: 600, color: "var(--text-tertiary)", marginBottom: 6, textTransform: "uppercase", letterSpacing: ".03em" }}>Display name</div>
          <input type="text" value={profile.displayName} onChange={(e) => setProfile((p) => ({ ...p, displayName: e.target.value }))} style={{ width: "100%" }} />
        </div>
        <div>
          <div style={{ fontSize: 11.5, fontWeight: 600, color: "var(--text-tertiary)", marginBottom: 6, textTransform: "uppercase", letterSpacing: ".03em" }}>Download location</div>
          <input type="text" value={profile.downloadPath} onChange={(e) => setProfile((p) => ({ ...p, downloadPath: e.target.value }))} style={{ width: "100%" }} />
        </div>
        <button className="btn btn-primary" style={{ alignSelf: "flex-start" }} onClick={() => toast("Profile saved")}>Save changes</button>
      </div>

      <div className="card" style={{ flex: 1, minWidth: 320, display: "flex", flexDirection: "column", gap: 4 }}>
        <div className="disp" style={{ fontSize: 15, fontWeight: 600, marginBottom: 4, display: "flex", alignItems: "center", gap: 8 }}>
          <Bell size={15} color="var(--text-tertiary)" /> Notifications
        </div>
        <SettingsRow label="Incoming file alerts" desc="Notify when someone sends you a file" on={profile.notifications.incoming} onClick={() => patchGroup("notifications", "incoming")} />
        <SettingsRow label="Transfer complete" desc="Notify when a transfer finishes" on={profile.notifications.complete} onClick={() => patchGroup("notifications", "complete")} />
        <SettingsRow label="Print job updates" desc="Notify on print job status changes" on={profile.notifications.print} onClick={() => patchGroup("notifications", "print")} />

        <div className="disp" style={{ fontSize: 15, fontWeight: 600, margin: "18px 0 4px", display: "flex", alignItems: "center", gap: 8 }}>
          <ShieldCheck size={15} color="var(--text-tertiary)" /> Confirmations
        </div>
        <SettingsRow label="Confirm incoming files" desc="Require approval before accepting files" on={profile.confirmations.files} onClick={() => patchGroup("confirmations", "files")} />
        <SettingsRow label="Confirm print jobs" desc="Require approval before printing" on={profile.confirmations.prints} onClick={() => patchGroup("confirmations", "prints")} />

        <div className="disp" style={{ fontSize: 15, fontWeight: 600, margin: "18px 0 4px", display: "flex", alignItems: "center", gap: 8 }}>
          <Printer size={15} color="var(--text-tertiary)" /> Printing
        </div>
        <SettingsRow label="Auto-accept trusted devices" desc="Skip approval for devices you've printed from before" on={profile.printPrefs.autoAccept} onClick={() => patchGroup("printPrefs", "autoAccept")} />
        <SettingsRow label="Print in background" desc="Don't bring the app to front for new jobs" on={profile.printPrefs.background} onClick={() => patchGroup("printPrefs", "background")} />
      </div>

      <div className="card" style={{ flex: 1, minWidth: 260 }}>
        <div className="disp" style={{ fontSize: 15, fontWeight: 600, marginBottom: 12, display: "flex", alignItems: "center", gap: 8 }}>
          <HardDrive size={15} color="var(--text-tertiary)" /> This device
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          {DEVICE_INFO.map(([k, v]) => (
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
