import React from "react";

// Native-style window title bar: traffic-light controls on the left,
// centered window title. This is the single strongest visual cue that
// separates "desktop app" from "web dashboard in a browser tab."
export default function TitleBar({ title }) {
  return (
    <div className="titlebar">
      <div className="traffic-lights">
        <span className="dot dot-red" />
        <span className="dot dot-yellow" />
        <span className="dot dot-green" />
      </div>
      <div className="titlebar-title mono">{title}</div>
    </div>
  );
}
