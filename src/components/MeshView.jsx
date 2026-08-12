import React from "react";
import { STATUS_META } from "../data/mockData";
import { initials } from "../utils/format";

// Signature visual: a radial "network mesh" with the local device at the
// center and peers arranged around it. Purely presentational — takes the
// same `devices` array as everything else.
export default function MeshView({ devices }) {
  const active = devices.filter((d) => !d.isSelf);
  const cx = 200, cy = 170, r = 130;
  return (
    <svg viewBox="0 0 400 320" style={{ width: "100%", height: 220 }}>
      <defs>
        <radialGradient id="meshGlow" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="var(--teal)" stopOpacity="0.16" />
          <stop offset="100%" stopColor="var(--teal)" stopOpacity="0" />
        </radialGradient>
      </defs>
      <circle cx={cx} cy={cy} r={r + 30} fill="url(#meshGlow)" />
      {active.map((d, i) => {
        const angle = (i / active.length) * Math.PI * 2 - Math.PI / 2;
        const x = cx + r * Math.cos(angle);
        const y = cy + r * Math.sin(angle) * 0.72;
        const meta = STATUS_META[d.status];
        const isLive = d.status === "online" || d.status === "transferring";
        return (
          <g key={d.id}>
            <line
              x1={cx} y1={cy} x2={x} y2={y}
              stroke={isLive ? meta.color : "var(--border)"}
              strokeWidth={isLive ? 1.4 : 1}
              opacity={isLive ? 0.55 : 0.3}
              className={d.status === "transferring" ? "mesh-line" : ""}
            />
            {d.status === "transferring" && (
              <circle cx={x} cy={y} r={20} fill="none" stroke={meta.color} strokeWidth="1" style={{ animation: "ringPulse 1.6s ease-in-out infinite" }} />
            )}
            <circle cx={x} cy={y} r={15} fill="var(--bg-elevated)" stroke={meta.color} strokeWidth="1.5" />
            <text x={x} y={y + 4} textAnchor="middle" fontSize="10" fontWeight="700" fill={meta.color} fontFamily="Space Grotesk, sans-serif">
              {initials(d.name)}
            </text>
            <text x={x} y={y + 28} textAnchor="middle" fontSize="9.5" fill="var(--text-tertiary)" fontFamily="Inter, sans-serif">
              {d.name}
            </text>
          </g>
        );
      })}
      <circle cx={cx} cy={cy} r={22} fill="var(--teal-dim)" stroke="var(--teal)" strokeWidth="1.5" />
      <text x={cx} y={cy + 4} textAnchor="middle" fontSize="10.5" fontWeight="700" fill="var(--teal)" fontFamily="Space Grotesk, sans-serif">YOU</text>
    </svg>
  );
}
