import React from "react";

// All design tokens (color, type, spacing) and shared class rules live here,
// scoped under .lan-app so this can be dropped into a larger page without
// leaking styles. Change a token once, every component picks it up.
//
// Tuned for a native desktop shell: flatter panels (hairlines over big
// rounded cards), a real title bar + status bar, tighter row density,
// smaller/quieter chrome than a typical web dashboard.
export default function GlobalStyle() {
  return (
    <style>{`
      @import url('https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;600;700&family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500;600&display=swap');

      html, 
      body { 
        margin: 0; 
        padding: 0; 
        width: 100%; 
        height: 100%; 
      }                
                        
      #root { 
        width: 100%; 
        height: 100%; 
      }   
      .lan-app {
        --bg: #0C1113;
        --bg-elevated: #12181B;
        --panel: #171E21;
        --panel-hover: #1D2528;
        --border: #262F33;
        --border-strong: #384347;
        --text-primary: #EAF1F1;
        --text-secondary: #93A6A9;
        --text-tertiary: #5D6E71;
        --teal: #3FDCC4;
        --teal-dim: #1D4A44;
        --teal-glow: rgba(63,220,196,0.16);
        --amber: #E8A63D;
        --amber-dim: #4A3A1D;
        --blue: #5B9FE0;
        --blue-dim: #1D3350;
        --red: #E8636B;
        --red-dim: #4A1F22;
        --gray: #5D6E71;
        font-family: 'Inter', sans-serif;
        background: var(--bg);
        color: var(--text-primary);
        height: 100vh;
        display: flex;
        flex-direction: column;
        position: relative;
        overflow: hidden;
        box-shadow: 0 0 0 1px var(--border), 0 24px 60px rgba(0,0,0,0.45);
        font-size: 13px;
      }
      .lan-app * { box-sizing: border-box; }
      .lan-app .disp { font-family: 'Space Grotesk', sans-serif; }
      .lan-app .mono { font-family: 'JetBrains Mono', monospace; }
      .lan-app ::-webkit-scrollbar { width: 7px; height: 7px; }
      .lan-app ::-webkit-scrollbar-thumb { background: var(--border-strong); border-radius: 8px; }
      .lan-app ::-webkit-scrollbar-track { background: transparent; }
      .lan-app button { font-family: inherit; cursor: pointer; }
      .lan-app input, .lan-app select { font-family: inherit; }

      /* ---- window chrome ---- */
      .lan-app .titlebar {
        height: 34px; flex-shrink: 0; position: relative;
        display: flex; align-items: center;
        background: var(--bg-elevated); border-bottom: 1px solid var(--border);
        padding: 0 12px; -webkit-app-region: drag;
      }
      .lan-app .traffic-lights { display: flex; gap: 7px; -webkit-app-region: no-drag; }
      .lan-app .dot { width: 11px; height: 11px; border-radius: 50%; }
      .lan-app .dot-red { background: #E8636B; }
      .lan-app .dot-yellow { background: #E8A63D; }
      .lan-app .dot-green { background: #3FDCC4; }
      .lan-app .titlebar-title {
        position: absolute; left: 50%; transform: translateX(-50%);
        font-size: 11.5px; color: var(--text-tertiary); letter-spacing: .03em;
      }

      .lan-app .toolbar {
        height: 42px; flex-shrink: 0; display: flex; align-items: center; gap: 10px;
        padding: 0 16px; border-bottom: 1px solid var(--border); background: var(--bg);
      }

      .lan-app .statusbar {
        height: 24px; flex-shrink: 0; display: flex; align-items: center; justify-content: space-between;
        padding: 0 14px; background: var(--bg-elevated); border-top: 1px solid var(--border);
        font-size: 10.5px; color: var(--text-tertiary);
      }

      /* ---- controls ---- */
      .lan-app .btn {
        display: inline-flex; align-items: center; gap: 7px;
        padding: 7px 13px; border-radius: var(--radius-sm);
        font-size: 12.5px; font-weight: 600; border: 1px solid transparent;
        transition: all .15s ease; white-space: nowrap;
      }
      .lan-app .btn-primary { background: var(--teal); color: #06201B; }
      .lan-app .btn-primary:hover { background: #57E6D0; }
      .lan-app .btn-primary:disabled { background: var(--border-strong); color: var(--text-tertiary); cursor: not-allowed; }
      .lan-app .btn-ghost { background: transparent; border-color: var(--border); color: var(--text-secondary); }
      .lan-app .btn-ghost:hover { border-color: var(--border-strong); color: var(--text-primary); background: var(--panel-hover); }
      .lan-app .btn-danger { background: transparent; border-color: var(--red-dim); color: var(--red); }
      .lan-app .btn-danger:hover { background: var(--red-dim); }
      .lan-app .btn-icon { padding: 6px; border-radius: var(--radius-sm); background: transparent; border: 1px solid var(--border); color: var(--text-secondary); display: inline-flex; }
      .lan-app .btn-icon:hover { color: var(--text-primary); border-color: var(--border-strong); background: var(--panel-hover); }

      .lan-app .card {
        background: var(--panel); border: 1px solid var(--border);
        border-radius: var(--radius-lg); padding: 14px 16px;
      }
      .lan-app .panel-head {
        display: flex; align-items: center; justify-content: space-between;
        padding: 0 2px 10px; margin-bottom: 10px; border-bottom: 1px solid var(--border);
      }
      .lan-app .chip {
        display: inline-flex; align-items: center; gap: 5px;
        font-size: 10.5px; font-weight: 600; padding: 2px 8px;
        border-radius: 4px; letter-spacing: .02em;
      }
      .lan-app .pulse-dot { width: 5px; height: 5px; border-radius: 50%; position: relative; }
      .lan-app .pulse-dot::after {
        content: ''; position: absolute; inset: -3px; border-radius: 50%;
        border: 1px solid currentColor; opacity: .5;
        animation: pulseRing 1.8s ease-out infinite;
      }
      @keyframes pulseRing { 0% { transform: scale(.6); opacity: .6; } 100% { transform: scale(2.2); opacity: 0; } }
      @keyframes fadeUp { from { opacity: 0; transform: translateY(4px); } to { opacity: 1; transform: translateY(0); } }
      .lan-app .fade-up { animation: fadeUp .2s ease both; }
      @keyframes toastIn { from { opacity: 0; transform: translateY(6px); } to { opacity: 1; transform: translateY(0); } }
      @keyframes dashMove { to { stroke-dashoffset: -24; } }
      .lan-app .mesh-line { stroke-dasharray: 3 5; animation: dashMove 2.2s linear infinite; }
      @keyframes ringPulse { 0%,100% { r: 20; opacity: .35; } 50% { r: 24; opacity: .1; } }

      .lan-app input[type="text"], .lan-app select {
        background: var(--bg-elevated); border: 1px solid var(--border);
        border-radius: var(--radius-sm); padding: 7px 10px; color: var(--text-primary);
        font-size: 12.5px; outline: none; transition: border-color .15s ease;
      }
      .lan-app input[type="text"]:focus, .lan-app select:focus { border-color: var(--teal); }
      .lan-app .toggle { width: 32px; height: 18px; border-radius: 100px; background: var(--border-strong); position: relative; transition: background .15s ease; flex-shrink:0; border:none; }
      .lan-app .toggle.on { background: var(--teal-dim); }
      .lan-app .toggle::after { content:''; position:absolute; top:2px; left:2px; width:14px; height:14px; border-radius:50%; background: var(--text-secondary); transition: all .15s ease; }
      .lan-app .toggle.on::after { left: 16px; background: var(--teal); }

      .lan-app .navlink {
        display:flex; align-items:center; gap:10px; padding:6px 10px; border-radius: var(--radius-sm);
        color: var(--text-secondary); font-size:12.5px; font-weight:500; transition: all .1s ease; position:relative;
      }
      .lan-app .navlink:hover { background: var(--panel-hover); color: var(--text-primary); }
      .lan-app .navlink.active { background: var(--panel); color: var(--text-primary); }
      .lan-app .navlink.active::before {
        content:''; position:absolute; left:-11px; top:50%; transform:translateY(-50%);
        width:3px; height:13px; background: var(--teal); border-radius: 3px;
      }
      .lan-app .nav-section-label {
        font-size: 10px; font-weight: 600; color: var(--text-tertiary); text-transform: uppercase;
        letter-spacing: .06em; padding: 12px 10px 5px;
      }

      .lan-app .progress-track { height:4px; border-radius:100px; background: var(--bg-elevated); overflow:hidden; border:1px solid var(--border); }
      .lan-app .progress-fill { height:100%; border-radius:100px; transition: width .3s ease; }

      .lan-app .device-row { display:flex; align-items:center; gap:10px; padding:7px 8px; border-radius: var(--radius-sm); border:1px solid transparent; transition: background .1s ease; }
      .lan-app .device-row:hover { background: var(--panel-hover); }
      .lan-app .device-row.selected { background: var(--teal-glow); border-color: var(--teal-dim); }

      .lan-app .avatar { width:30px; height:30px; border-radius:7px; display:flex; align-items:center; justify-content:center; font-weight:700; font-size:11.5px; flex-shrink:0; }
      .lan-app .scrollarea { overflow-y:auto; }
      .lan-app .empty-state { display:flex; flex-direction:column; align-items:center; justify-content:center; padding:36px 20px; text-align:center; color: var(--text-tertiary); gap:8px; font-size: 12.5px; }
      .lan-app .dropzone { border:1.5px dashed var(--border-strong); border-radius: var(--radius-md); padding:26px 20px; text-align:center; transition: all .15s ease; }
      .lan-app .dropzone.active { border-color: var(--teal); background: var(--teal-glow); }
      .lan-app .tab { padding:6px 3px; font-size:12.5px; font-weight:600; color: var(--text-tertiary); border-bottom:2px solid transparent; }
      .lan-app .tab.active { color: var(--text-primary); border-color: var(--teal); }

      .lan-app .strip { display:flex; border: 1px solid var(--border); border-radius: var(--radius-lg); overflow: hidden; background: var(--panel); }
      .lan-app .strip-seg { flex: 1; padding: 10px 16px; border-right: 1px solid var(--border); }
      .lan-app .strip-seg:last-child { border-right: none; }

      .lan-app .toast {
        animation: toastIn .18s ease both;
        background: var(--bg-elevated); border: 1px solid var(--border-strong);
        border-radius: var(--radius-sm); padding: 8px 11px; display: flex; gap: 8px; align-items: flex-start;
        box-shadow: 0 6px 16px rgba(0,0,0,0.4); font-size: 11.5px;
      }
    `}</style>
  );
}
