import React from "react";

// Controlled on/off switch. Fully stateless — parent owns the value.
export default function Toggle({ on, onClick }) {
  return <button className={`toggle ${on ? "on" : ""}`} onClick={onClick} aria-pressed={on} />;
}
