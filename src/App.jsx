import React, { useState, useCallback, useEffect } from "react";

import GlobalStyle from "./styles/GlobalStyle";
import Sidebar from "./components/Sidebar";
import TopBar from "./components/TopBar";
import ToastStack from "./components/ToastStack";
import ConfirmModal from "./components/ConfirmModal";

import DashboardView from "./views/DashboardView";
import SendFilesView from "./views/SendFilesView";
import TransfersView from "./views/TransfersView";
import PrintView from "./views/PrintView";
import NetworkView from "./views/NetworkView";
import SettingsView from "./views/SettingsView";

import { initialDevices, initialTransfers, initialPrintJobs } from "./data/mockData";
import { fmtBytes } from "./utils/format";

/**
 * App is the only place state lives and the only place that knows how
 * views are wired together. Every view/component below it is a pure
 * function of props — swap mock data for real device discovery, a real
 * transfer queue, etc. by changing what's passed in here, nothing else.
 */
export default function App() {
  const [view, setView] = useState("dashboard");
  const [devices] = useState(initialDevices);
  const [transfers, setTransfers] = useState(initialTransfers);
  const [printJobs, setPrintJobs] = useState(initialPrintJobs);
  const [toasts, setToasts] = useState([]);
  const [modal, setModal] = useState(null);
  const [network, setNetwork] = useState({ mode: "host", connected: true, sessionName: "Home-Studio-5G", sessionCode: "FIR-482-QLT" });
  const [profile, setProfile] = useState({
    displayName: "You",
    avatarColor: "#3FDCC4",
    downloadPath: "~/Downloads/LANShare",
    notifications: { incoming: true, complete: true, print: false },
    confirmations: { files: true, prints: true },
    printPrefs: { autoAccept: false, background: true },
  });

  const toast = useCallback((message, error) => {
    const id = Math.random().toString(36).slice(2);
    setToasts((t) => [...t, { id, message, error }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 3400);
  }, []);

  // Simulated progress ticks — replace with real transfer/print events.
  useEffect(() => {
    const iv = setInterval(() => {
      setTransfers((prev) => prev.map((t) => {
        if (t.status !== "active") return t;
        const next = Math.min(100, t.progress + Math.random() * 9);
        if (next >= 100) return { ...t, progress: 100, status: "completed" };
        return { ...t, progress: Math.round(next) };
      }));
    }, 1100);
    return () => clearInterval(iv);
  }, []);

  useEffect(() => {
    const iv = setInterval(() => {
      setPrintJobs((prev) => prev.map((p) => (p.status === "printing" && Math.random() > 0.7 ? { ...p, status: "completed" } : p)));
    }, 2600);
    return () => clearInterval(iv);
  }, []);

  const handleTransferAction = (id, action) => {
    setTransfers((prev) => prev.reduce((acc, t) => {
      if (t.id !== id) return [...acc, t];
      if (action === "remove") return acc;
      if (action === "pause") return [...acc, { ...t, status: "paused" }];
      if (action === "resume") return [...acc, { ...t, status: "active" }];
      if (action === "retry") return [...acc, { ...t, status: "active", progress: 0, error: undefined }];
      if (action === "cancel") return [...acc, { ...t, status: "failed", error: "Cancelled" }];
      return [...acc, t];
    }, []));
  };

  const handleSend = (files, names) => {
    const newTransfers = files.map((f, i) => ({
      id: `t${Date.now()}${i}`,
      file: f.name,
      size: fmtBytes(f.size),
      target: names[0] + (names.length > 1 ? ` +${names.length - 1}` : ""),
      direction: "send",
      status: "active",
      progress: 0,
      speed: "— MB/s",
      eta: "—",
    }));
    setTransfers((prev) => [...newTransfers, ...prev]);
  };

  const handlePrint = (doc, target) => {
    setPrintJobs((prev) => [
      { id: `p${Date.now()}`, file: doc.name, target: target.name, printer: target.printerName, status: "pending", pages: Math.ceil(Math.random() * 6) },
      ...prev,
    ]);
  };

  const handleJobAction = (id, status) => {
    setPrintJobs((prev) => prev.map((p) => (p.id === id ? { ...p, status } : p)));
  };

  return (
    <div className="lan-app">
      <GlobalStyle />
      <Sidebar view={view} setView={setView} profile={profile} network={network} />

      <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
        <TopBar view={view} network={network} onSend={() => setView("send")} />

        <div className="scrollarea" style={{ padding: "24px 28px", flex: 1 }}>
          {view === "dashboard" && <DashboardView devices={devices} transfers={transfers} printJobs={printJobs} network={network} setView={setView} />}
          {view === "send" && <SendFilesView devices={devices} onSend={handleSend} toast={toast} />}
          {view === "transfers" && <TransfersView transfers={transfers} onAction={handleTransferAction} />}
          {view === "print" && <PrintView devices={devices} printJobs={printJobs} onPrint={handlePrint} onJobAction={handleJobAction} toast={toast} />}
          {view === "network" && <NetworkView network={network} setNetwork={setNetwork} devices={devices} toast={toast} confirm={setModal} />}
          {view === "settings" && <SettingsView profile={profile} setProfile={setProfile} toast={toast} />}
        </div>
      </div>

      <ToastStack toasts={toasts} />
      <ConfirmModal modal={modal} onClose={() => setModal(null)} />
    </div>
  );
}
