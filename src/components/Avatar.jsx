import React from "react";
import { initials } from "../utils/format";

// Colored initials avatar. `device` needs { name, color }.
export default function Avatar({ device, size = 36 }) {
  return (
    <div
      className="avatar"
      style={{
        width: size,
        height: size,
        background: `${device.color}22`,
        color: device.color,
        border: `1px solid ${device.color}44`,
      }}
    >
      {initials(device.name)}
    </div>
  );
}
