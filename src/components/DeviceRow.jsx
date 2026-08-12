import React from "react";
import Avatar from "./Avatar";
import StatusChip from "./StatusChip";

// Reusable device list row — used by Dashboard, Send, Print, and Network views.
// `trailing` lets each view swap in its own right-side control (checkbox,
// printer icon, remove button, etc.) without forking this component.
export default function DeviceRow({ device, size = 36, selected = false, onClick, trailing, subtitle }) {
  return (
    <div
      className={`device-row ${selected ? "selected" : ""}`}
      onClick={onClick}
      style={{ cursor: onClick ? "pointer" : "default" }}
    >
      <Avatar device={device} size={size} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 13.5, fontWeight: 600 }}>{device.name}</div>
        <div className="mono" style={{ fontSize: 11.5, color: "var(--text-tertiary)" }}>
          {subtitle || device.device}
        </div>
      </div>
      {trailing !== undefined ? trailing : <StatusChip status={device.status} />}
    </div>
  );
}
