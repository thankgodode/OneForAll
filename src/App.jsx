import React, { useState, useCallback, useEffect } from "react";

import GlobalStyle from "./styles/GlobalStyle";
import TitleBar from "./components/TitleBar";
import Sidebar from "./components/Sidebar";
import TopBar from "./components/TopBar";
import StatusBar from "./components/StatusBar";
import ToastStack from "./components/ToastStack";
import ConfirmModal from "./components/ConfirmModal";

import DashboardView from "./views/DashboardView";
import SendFilesView from "./views/SendFilesView";
import TransfersView from "./views/TransfersView";
import PrintView from "./views/PrintView";
import NetworkView from "./views/NetworkView";
import SettingsView from "./views/SettingsView";

import { initialPrintJobs } from "./data/mockData"; // printing has no backend yet — stays mocked
import { HOST_PEER_ID } from "./data/protocol";
import { fmtBytes, fmtSpeed, fmtEta } from "./utils/format";
import { toUiDevice } from "./utils/peer";

import * as api from "./api/commands";
import { subscribeToBackendEvents } from "./api/events";

const emptyNetwork = {
  role: "none", // "none" | "host" | "joined"
  connected: false,
  sessionName: "",
  myPeerId: null,
  hostAddr: null,
  joinStatus: "idle", // "idle" | "connecting" | "pending"
};

/**
 * App is the only place state lives and the only place that knows how
 * views are wired together, AND the only place that talks to the Tauri
 * backend — every view below it is still a pure function of props, now
 * fed by real commands/events instead of mock data.
 */
export default function App() {
  const [view, setView] = useState("dashboard");
  const [deviceName, setDeviceName] = useState("This device");
  const [myListenPort, setMyListenPort] = useState(null);

  const [network, setNetwork] = useState(emptyNetwork);
  const [roster, setRoster] = useState([]); // PeerInfo[], excludes self
  const [discoveredHosts, setDiscoveredHosts] = useState([]); // [{fullname, addr}]
  const [pendingJoins, setPendingJoins] = useState([]); // PeerInfo[] awaiting host approval
  const [pendingIncomingFiles, setPendingIncomingFiles] = useState([]); // IncomingFileRequest[]
  const [accessControl, setAccessControl] = useState({ requireApproval: true, joinCode: "" });

  const [transfers, setTransfers] = useState([]);
  const [printJobs, setPrintJobs] = useState(initialPrintJobs);
  const [toasts, setToasts] = useState([]);
  const [modal, setModal] = useState(null);

  const [profile, setProfile] = useState({
    displayName: "You",
    avatarColor: "#3FDCC4",
    downloadPath: "",
    notifications: { incoming: true, complete: true, print: false },
    confirmations: { files: true, prints: true },
    printPrefs: { autoAccept: false, background: true },
  });

  const toast = useCallback((message, error) => {
    const id = Math.random().toString(36).slice(2);
    setToasts((t) => [...t, { id, message, error }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 3400);
  }, []);

  // A modal descriptor can carry an optional onCancel — ConfirmModal
  // itself only knows "confirm" vs "close"; this is what lets closing
  // (rather than confirming) an incoming-file prompt count as a reject.
  const closeModal = useCallback(() => {
    const closing = modal;
    setModal(null);
    closing?.onCancel?.();
  }, [modal]);

  const updateTransfer = useCallback((transferId, patch) => {
    setTransfers((prev) => prev.map((t) => (t.id === transferId ? { ...t, ...patch } : t)));
  }, []);

  /* ---------------- startup: identity, listening, discovery ---------------- */
  useEffect(() => {
    let unsubscribe;

    (async () => {
      try {
        const name = await api.getDeviceName();
        setDeviceName(name);
        setProfile((p) => ({ ...p, displayName: name }));
      } catch {
        // keep the "This device" default — not fatal
      }

      try {
        const port = await api.startListening();
        setMyListenPort(port);
      } catch {
        toast("Couldn't start listening for connections", true);
      }

      try {
        await api.browseService();
      } catch {
        toast("Couldn't start network discovery", true);
      }

      unsubscribe = await subscribeToBackendEvents({
        onServiceResolved: (fullname, addr) => {
          if (!addr) return;
          setDiscoveredHosts((prev) => {
            const others = prev.filter((h) => h.fullname !== fullname);
            return [...others, { fullname, addr }];
          });
        },
        onServiceRemoved: (fullname) => {
          setDiscoveredHosts((prev) => prev.filter((h) => h.fullname !== fullname));
        },

        onJoinRequested: (peer) => {
          setPendingJoins((prev) => {
            if (prev.some((p) => p.id === peer.id)) {
              return prev;
            }

            return [...prev, peer];
          });
        },
        onJoinPending: () => {
          setNetwork((n) => ({ ...n, joinStatus: "pending" }));
        },
        onJoinedNetwork: (sessionName, hostInfo, initialRoster) => {
          setNetwork((n) => ({ ...n, role: "joined", connected: true, sessionName, joinStatus: "idle" }));
          // The host is never part of `roster` itself (see control.rs —
          // it never inserts itself as a member of its own session), so
          // it's merged in here — otherwise a joined client's device
          // list would show every peer except the one it's actually
          // connected to.
          setRoster([{ ...hostInfo, isHost: true }, ...initialRoster]);
        },
        onPeerJoined: (peer) => {
          setRoster((prev) => (prev.some((p) => p.id === peer.id) ? prev : [...prev, peer]));
        },
        onPeerLeft: (peerId) => {
          setRoster((prev) => prev.filter((p) => p.id !== peerId));
        },
        onKicked: (reason) => {
          setNetwork(emptyNetwork);
          setRoster([]);
          toast(`Removed from network: ${reason}`, true);
        },
        onDisconnected: () => {
          setNetwork(emptyNetwork);
          setRoster([]);
        },

        onIncomingFileRequest: (req) => {
          setPendingIncomingFiles((prev) => [...prev, req]);
        },
        onTransferStarted: (t) => {
          setTransfers((prev) => [
            {
              id: t.transferId,
              file: t.filename,
              sizeBytes: t.size,
              size: fmtBytes(t.size),
              target: t.peerName,
              peerId: t.peerId,
              direction: t.direction,
              status: "active",
              progress: 0,
              speed: "—",
              eta: "—",
            },
            ...prev,
          ]);
        },
        onTransferProgress: (p) => {
          const pct = p.totalBytes > 0 ? Math.round((p.bytesDone / p.totalBytes) * 100) : 0;
          updateTransfer(p.transferId, {
            progress: pct,
            speed: fmtSpeed(p.speedBytesPerSec),
            eta: fmtEta(p.bytesDone, p.totalBytes, p.speedBytesPerSec),
          });
        },
        onTransferCompleted: (transferId) => {
          updateTransfer(transferId, { status: "completed", progress: 100 });
        },
        onTransferFailed: (transferId, reason) => {
          updateTransfer(transferId, { status: "failed", error: reason });
        },
        onTransferCancelled: (transferId) => {
          updateTransfer(transferId, { status: "failed", error: "Cancelled" });
        },
        onTransferRejected: (transferId) => {
          updateTransfer(transferId, { status: "failed", error: "Rejected by recipient" });
        },
      });
    })();

    return () => unsubscribe?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Show incoming-file prompts one at a time via the shared ConfirmModal.
  useEffect(() => {
    if (modal || pendingIncomingFiles.length === 0) return;
    const req = pendingIncomingFiles[0];
    const resolve = (accept) => {
      api.respondToIncomingFile(req.transferId, accept).catch(() => toast("Failed to respond to file request", true));
      setPendingIncomingFiles((prev) => prev.filter((r) => r.transferId !== req.transferId));
    };
    setModal({
      title: `Incoming file: ${req.filename}`,
      body: `${req.from?.display_name || "Someone"} wants to send you ${fmtBytes(req.size)}.`,
      confirmLabel: "Accept",
      onConfirm: () => resolve(true),
      onCancel: () => resolve(false),
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pendingIncomingFiles, modal]);

  /* ---------------- transfers ---------------- */

  const handleSend = async (filePaths, targetPeerIds) => {
    if (!network.myPeerId) {
      toast("Join or host a network before sending files", true);
      return;
    }
    for (const path of filePaths) {
      try {
        await api.sendFileToPeers(network.myPeerId, path, targetPeerIds);
      } catch (e) {
        toast(`Failed to send ${path.split("/").pop()}: ${e}`, true);
      }
    }
  };

  const handleTransferAction = (id, action) => {
    if (action === "cancel") {
      api.cancelTransfer(id).catch(() => toast("Failed to cancel transfer", true));
      return;
    }
    if (action === "remove") {
      setTransfers((prev) => prev.filter((t) => t.id !== id));
      return;
    }
    if (action === "pause" || action === "resume") {
      toast("Pausing isn't supported yet — cancel and resend instead");
      return;
    }
    if (action === "retry") {
      // The backend never hands back the original file path for a
      // completed call, so a true one-click retry isn't possible without
      // re-selecting the file — this is the honest version of that.
      setTransfers((prev) => prev.filter((t) => t.id !== id));
      setView("send");
      toast("Select the file again to resend it");
    }
  };

  /* ---------------- printing (still mocked — no backend support yet) ---------------- */

  const handlePrint = (doc, target) => {
    setPrintJobs((prev) => [
      { id: `p${Date.now()}`, file: doc.name, target: target.name, printer: target.printerName, status: "pending", pages: Math.ceil(Math.random() * 6) },
      ...prev,
    ]);
  };
  const handleJobAction = (id, status) => {
    setPrintJobs((prev) => prev.map((p) => (p.id === id ? { ...p, status } : p)));
  };

  /* ---------------- network: hosting ---------------- */

  const handleStartHosting = async (sessionName) => {
    if (myListenPort == null) {
      toast("Still starting up — try again in a moment", true);
      return;
    }
    try {
      await api.startHosting(sessionName, deviceName);
      setNetwork({ ...emptyNetwork, role: "host", connected: true, sessionName, myPeerId: HOST_PEER_ID });
    } catch (e) {
      toast(`Couldn't start hosting: ${e}`, true);
    }
  };

  const handleStopHosting = async () => {
    try {
      await api.stopHosting();
    } catch (e) {
      toast(`Couldn't stop hosting cleanly: ${e}`, true);
    }
    setNetwork(emptyNetwork);
    setRoster([]);
  };

  const handleSetAccessControl = async (requireApproval, joinCode) => {
    try {
      await api.setAutoAccept(!requireApproval);
      await api.setJoinCode(joinCode || null);
      setAccessControl({ requireApproval, joinCode });
    } catch {
      toast("Failed to update access settings", true);
    }
  };

  const handleRespondToJoin = (requestId, accept) => {
    api.respondToJoin(requestId, accept).catch(() => toast("Failed to respond to join request", true));
    setPendingJoins((prev) => prev.filter((p) => p.id !== requestId));
  };

  const handleKick = (peerId) => {
    api.kickPeer(peerId).catch(() => toast("Failed to remove device", true));
  };

  /* ---------------- network: joining ---------------- */

  const handleJoin = async (hostAddr) => {
    if (myListenPort == null) {
      toast("Still starting up — try again in a moment", true);
      return;
    }
    setNetwork((n) => ({ ...n, joinStatus: "connecting", hostAddr }));
    try {
      const me = await api.joinNetwork(hostAddr, profile.displayName, myListenPort);
      setNetwork((n) => ({ ...n, myPeerId: me.id }));
    } catch (e) {
      setNetwork(emptyNetwork);
      toast(`Couldn't join network: ${e}`, true);
    }
  };

  const handleLeaveNetwork = () => {
    api.leaveNetwork().catch(() => {});
    setNetwork(emptyNetwork);
    setRoster([]);
  };

  /* ---------------- settings ---------------- */

  const handleSetConfirmFiles = (value) => {
    setProfile((p) => ({ ...p, confirmations: { ...p.confirmations, files: value } }));
    api.setConfirmIncomingFiles(value).catch(() => toast("Failed to save setting", true));
  };

  const handleSetDownloadDir = (path) => {
    setProfile((p) => ({ ...p, downloadPath: path }));
    api.setDownloadDir(path).catch(() => toast("Failed to save download location", true));
  };

  /* ---------------- derived device list ---------------- */

  const transferringPeerIds = new Set(transfers.filter((t) => t.status === "active").map((t) => t.peerId));
  const selfDevice = {
    id: network.myPeerId || "you",
    name: profile.displayName || "You",
    device: deviceName,
    status: "online",
    isSelf: true,
    hasPrinter: false,
    color: profile.avatarColor,
  };
  const devices = [
    selfDevice,
    ...roster.map((p) => toUiDevice(p, { status: transferringPeerIds.has(p.id) ? "transferring" : "online" })),
  ];

  return (
    <div className="lan-app">
      <GlobalStyle />
      <TitleBar title="Nearby" />

      <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
        <Sidebar view={view} setView={setView} profile={profile} network={network} />

        <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
          <TopBar view={view} network={network} onSend={() => setView("send")} />

          <div className="scrollarea" style={{ padding: "16px 20px", flex: 1 }}>
            {view === "dashboard" && (
              <DashboardView devices={devices} transfers={transfers} printJobs={printJobs} network={network} setView={setView} />
            )}
            {view === "send" && <SendFilesView devices={devices} onSend={handleSend} toast={toast} />}
            {view === "transfers" && <TransfersView transfers={transfers} onAction={handleTransferAction} />}
            {view === "print" && (
              <PrintView devices={devices} printJobs={printJobs} onPrint={handlePrint} onJobAction={handleJobAction} toast={toast} />
            )}
            {view === "network" && (
              <NetworkView
                network={network}
                deviceName={deviceName}
                discoveredHosts={discoveredHosts}
                pendingJoins={pendingJoins}
                devices={devices}
                accessControl={accessControl}
                onStartHosting={handleStartHosting}
                onStopHosting={handleStopHosting}
                onJoin={handleJoin}
                onLeaveNetwork={handleLeaveNetwork}
                onRespondToJoin={handleRespondToJoin}
                onKick={handleKick}
                onSetAccessControl={handleSetAccessControl}
                toast={toast}
                confirm={setModal}
              />
            )}
            {view === "settings" && (
              <SettingsView
                profile={profile}
                setProfile={setProfile}
                onSetConfirmFiles={handleSetConfirmFiles}
                onSetDownloadDir={handleSetDownloadDir}
                deviceName={deviceName}
                myListenPort={myListenPort}
                toast={toast}
              />
            )}
          </div>
        </div>
      </div>

      <StatusBar network={network} devices={devices} transfers={transfers} />
      <ToastStack toasts={toasts} />
      <ConfirmModal modal={modal} onClose={closeModal} />
    </div>
  );
}
