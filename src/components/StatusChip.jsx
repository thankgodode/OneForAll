import React from "react";
import { STATUS_META } from "../data/mockData";

// Pill with a pulsing dot showing a device's connection status.
export default function StatusChip({ status }) {
  const m = STATUS_META[status];
  if (!m) return null;
  return (
    <span className="chip" style={{ background: `${m.color}1A`, color: m.color }}>
      <span className="pulse-dot" style={{ background: m.color, color: m.color }} />
      {m.label}
    </span>
  );
}
