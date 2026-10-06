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

import { HOST_PEER_ID } from "./data/protocol";
import { fmtBytes, fmtSpeed, fmtEta, fmtTimeAgo } from "./utils/format";
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
  const [localPrinters, setLocalPrinters] = useState([]);
  const [myListenPort, setMyListenPort] = useState(null);

  const [network, setNetwork] = useState(emptyNetwork);
  const [roster, setRoster] = useState([]); // PeerInfo[], excludes self
  const [discoveredHosts, setDiscoveredHosts] = useState([]); // [{fullname, addr}]
  const [pendingJoins, setPendingJoins] = useState([]); // PeerInfo[] awaiting host approval
  const [pendingIncomingFiles, setPendingIncomingFiles] = useState([]); // IncomingFileRequest[]
  const [accessControl, setAccessControl] = useState({ requireApproval: true, joinCode: "" });

  const [transfers, setTransfers] = useState([]);
  const [printJobs, setPrintJobs] = useState([]);
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
    // The `cancelled` flag guards against React's dev-mode double-invoke
    // of effects (mount → cleanup → mount again, to surface exactly this
    // class of bug). Without it: cleanup can run before this async body
    // has assigned `unsubscribe` at all (the assignment happens after an
    // `await`), so the early cleanup call is a no-op on a variable that's
    // still `undefined` — the first run's listeners never get torn down,
    // and the second run adds a full second set on top. Every event
    // handler then fires twice per real event, which is what duplicate
    // rows in the UI (e.g. the same join request appearing twice) look
    // like from the outside. Checking `cancelled` after each await lets
    // a stale run unsubscribe itself the moment it notices it's stale,
    // instead of leaving a second live subscription behind.
    let cancelled = false;
    let unsubscribe = null;

    (async () => {
      try {
        const name = await api.getDeviceName();
        if (cancelled) return;
        setDeviceName(name);
      } catch {
        // keep the "This device" default — not fatal
      }

      try {
        const printers = await api.getPrinters();
        if (cancelled) return;
        setLocalPrinters(printers);
      } catch {
        if (!cancelled) toast("Couldn't detect this device's printers", true);
      }

      // Restores whatever was saved last session — display name, avatar
      // color, download dir, confirm-files preference all persist now
      // (see nearby-backend/src/db.rs), instead of resetting every launch.
      try {
        const settings = await api.getSettings();
        if (cancelled) return;
        setProfile((p) => ({
          ...p,
          displayName: settings.displayName,
          avatarColor: settings.avatarColor,
          downloadPath: settings.downloadDir,
          confirmations: { ...p.confirmations, files: settings.confirmIncomingFiles },
        }));
      } catch {
        // fresh install / DB read failed — the defaults already in state are fine
      }

      try {
        const port = await api.startListening();
        if (cancelled) return;
        setMyListenPort(port);
      } catch {
        if (!cancelled) toast("Couldn't start listening for connections", true);
      }

      try {
        await api.browseService();
      } catch {
        if (!cancelled) toast("Couldn't start network discovery", true);
      }

      // Interrupted transfers from a PREVIOUS session (not just ones that
      // failed during this one) — the ledger backing this is on disk, so
      // it survives a full app restart, not just a still-running reconnect.
      try {
        const incomplete = await api.listIncompleteTransfers();
        if (!cancelled && incomplete.length) {
          setTransfers((prev) => {
            const existingIds = new Set(prev.map((t) => t.id));
            const restored = incomplete
              .filter((t) => !existingIds.has(t.transferId))
              .map((t) => {
                // interruptedAt can be null if the app crashed mid-stream
                // rather than stopping through the normal cleanup path —
                // createdAt is the honest fallback for "how long ago"
                // rather than showing nothing.
                const since = t.interruptedAt ?? t.createdAt;
                return {
                  id: t.transferId,
                  file: t.filename,
                  sizeBytes: t.totalSize,
                  size: fmtBytes(t.totalSize),
                  target: t.peerDisplayName,
                  peerId: null, // re-resolved by device name when actually resumed — see resume_transfer
                  direction: t.direction,
                  status: "failed",
                  progress: 0,
                  speed: "—",
                  eta: "—",
                  error: `Interrupted ${fmtTimeAgo(since)} — resume once reconnected`,
                };
              });
            return [...restored, ...prev];
          });
        }
      } catch {
        // not fatal — just means no restored rows this launch
      }

      if (cancelled) return;

      const unsub = await subscribeToBackendEvents({
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
          setPendingJoins((prev) => (prev.some((p) => p.id === peer.id) ? prev : [...prev, peer]));
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
        onPeerLeft: (peerId, reason) => {
          setRoster((prev) => {
            const peer = prev.find((p) => p.id === peerId);
            // Reason-aware messaging is the whole point of the backend
            // change: "left" is routine and doesn't need a toast at all;
            // "timedOut" is the one worth flagging, since it's the only
            // case where "gone" might just be temporary.
            if (peer && reason === "timedOut") toast(`Lost connection to ${peer.displayName}`, true);
            return prev.filter((p) => p.id !== peerId);
          });
        },
        onKicked: (reason) => {
          setNetwork(emptyNetwork);
          setRoster([]);
          toast(`Removed from network: ${reason}`, true);
        },
        onDisconnected: (reason) => {
          setNetwork(emptyNetwork);
          setRoster([]);
          // In practice this is always "timedOut" — an explicit Leave or
          // Kick already resolves through their own dedicated events
          // before this one would ever fire (see join.rs) — but matching
          // on it rather than assuming keeps this honest if that ever
          // changes on the backend.
          toast(reason === "timedOut" ? "Disconnected from the network — reconnect once you're back" : "Disconnected from the network", true);
        },

        onIncomingFileRequest: (req) => {
          setPendingIncomingFiles((prev) => (prev.some((r) => r.transferId === req.transferId) ? prev : [...prev, req]));
          if (req.printerName) {
            setPrintJobs((prev) => prev.some((job) => job.id === req.transferId) ? prev : [{
              id: req.transferId,
              file: req.filename,
              target: "This device",
              printer: req.printerName,
              status: "pending",
              direction: "receive",
            }, ...prev]);
          }
        },
        onTransferStarted: (t) => {
          setTransfers((prev) => (
            prev.some((x) => x.id === t.transferId) ? prev : [
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
            ]
          ));
          if (t.printerName) {
            setPrintJobs((prev) => {
              const job = {
                id: t.transferId,
                file: t.filename,
                target: t.direction === "send" ? t.peerName : "This device",
                printer: t.printerName,
                status: t.direction === "send" ? "pending" : "printing",
                direction: t.direction,
              };
              return prev.some((item) => item.id === t.transferId)
                ? prev.map((item) => item.id === t.transferId ? { ...item, ...job } : item)
                : [job, ...prev];
            });
          }
        },
        // Fires on both a successful reconnect-and-continue AND (should
        // it ever arrive for a row that isn't already in state) a
        // defensive no-op — but in practice the row always exists first,
        // either from onTransferStarted this session or from the
        // listIncompleteTransfers() restore at startup.
        onTransferResumed: (transferId, resumedFromBytes) => {
          setTransfers((prev) => prev.map((t) => (
            t.id === transferId
              ? {
                  ...t,
                  status: "active",
                  error: undefined,
                  progress: t.sizeBytes > 0 ? Math.round((resumedFromBytes / t.sizeBytes) * 100) : 0,
                }
              : t
          )));
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
          setPrintJobs((prev) => prev.map((job) => job.id === transferId
            ? { ...job, status: job.direction === "send" ? "sent" : "completed" }
            : job));
        },
        onTransferFailed: (transferId, reason) => {
          updateTransfer(transferId, { status: "failed", error: reason });
          setPrintJobs((prev) => prev.map((job) => job.id === transferId ? { ...job, status: "failed", error: reason } : job));
        },
        onTransferCancelled: (transferId) => {
          updateTransfer(transferId, { status: "failed", error: "Cancelled" });
          setPrintJobs((prev) => prev.map((job) => job.id === transferId ? { ...job, status: "failed" } : job));
        },
        onTransferRejected: (transferId) => {
          updateTransfer(transferId, { status: "failed", error: "Rejected by recipient" });
          setPrintJobs((prev) => prev.map((job) => job.id === transferId ? { ...job, status: "rejected" } : job));
        },
      });

      // If cleanup already ran by the time this resolved (the exact
      // double-invoke race described above), tear this stale set of
      // listeners down immediately instead of leaving it live.
      if (cancelled) {
        unsub();
      } else {
        unsubscribe = unsub;
      }
    })();

    return () => {
      cancelled = true;
      unsubscribe?.();
    };
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
      title: req.printerName ? `Print job: ${req.filename}` : `Incoming file: ${req.filename}`,
      body: req.printerName
        ? `${req.from?.displayName || "Someone"} wants to send ${req.filename} (${fmtBytes(req.size)}) to print on ${req.printerName}.`
        : `${req.from?.displayName || "Someone"} wants to send you ${fmtBytes(req.size)}.`,
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
      api.removeIncomplete(id).catch((e) => { 
        console.log("Error: ", e); 
        toast("Failed to remove incomplete file", true) 
      })
      setTransfers((prev) => prev.filter((t) => t.id !== id));
      return;
    }
    if (action === "pause") {
      toast("Pausing isn't supported yet — cancel and resend instead");
      return;
    }
    if (action === "resume") {
      if (!network.myPeerId) {
        toast("Join or host a network before resuming a transfer", true);
        return;
      }
      const t = transfers.find((x) => x.id === id);
      // Direction decides which side initiates: if we were the receiver,
      // we already know our own partial file's size, so we reach out
      // directly (requestResume) rather than waiting for the original
      // sender to remember and reconnect (resumeTransfer). No need to
      // check WHICH kind of failure this was beyond that — either
      // command errors cleanly ("No record of this transfer") if
      // there's nothing to continue, e.g. a send that never actually
      // connected in the first place.
      // const resume = t?.direction === "receive" ? api.requestResume : api.resumeTransfer;
      const resume = api.requestResume;
      resume(network.myPeerId, id).catch((e) => toast(`Couldn't resume: ${e}`, true));
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

  /* ---------------- printing ---------------- */

  const handlePrint = async (doc, target, printerName) => {
    if (!network.myPeerId) throw new Error("Join or host a network before sending print jobs");
    const id = await api.sendPrintJob(network.myPeerId, doc.path, target.id, printerName);
    setPrintJobs((prev) => prev.some((job) => job.id === id) ? prev : [{
      id,
      file: doc.name,
      target: target.name,
      printer: printerName,
      status: "pending",
      direction: "send",
    }, ...prev]);
    toast(`Print job sent to ${target.name} (${printerName})`);
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
      const me = await api.joinNetwork(hostAddr, profile.displayName, myListenPort, localPrinters);
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

  const handleSetDisplayName = (name) => {
    setProfile((p) => ({ ...p, displayName: name }));
    api.setDisplayName(name).catch(() => toast("Failed to save display name", true));
  };

  const handleSetAvatarColor = (color) => {
    setProfile((p) => ({ ...p, avatarColor: color }));
    api.setAvatarColor(color).catch(() => toast("Failed to save avatar color", true));
  };

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
    hasPrinter: localPrinters.length > 0,
    printerNames: localPrinters,
    color: profile.avatarColor,
  };
  const devices = [
    selfDevice,
    ...roster.map((p) => toUiDevice(p, { status: transferringPeerIds.has(p.id) ? "transferring" : "online" })),
  ];

  return (
    <div className="lan-app">
      <GlobalStyle />
      {/* <TitleBar title="Nearby" /> */}

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
              <PrintView devices={devices} printJobs={printJobs} onPrint={handlePrint} toast={toast} />
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
                onSetDisplayName={handleSetDisplayName}
                onSetAvatarColor={handleSetAvatarColor}
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
