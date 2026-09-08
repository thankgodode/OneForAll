import { listen } from "@tauri-apps/api/event";

// Every event name the backend emits, and the payload shape that arrives
// with it (see nearby-backend/src/events.rs and the emit() calls in
// network/*.rs — this list has to stay in sync with those, by hand,
// since there's no shared schema between the two languages).
//
//   mdns-service-resolved     (fullname: string, addr: string | null)
//   mdns-service-removed      fullname: string
//   join-requested            PeerInfo                  — host: someone wants in
//   join-pending              ()                         — client: waiting on host
//   joined-network            (sessionName: string, hostInfo: PeerInfo, roster: PeerInfo[])
//   peer-joined                PeerInfo
//   peer-left                   peerId: string
//   kicked                       reason: string
//   disconnected-from-network     ()
//   incoming-file-request           IncomingFileRequest
//   transfer-started                  { transferId, peerId, peerName, filename, size, direction }
//   transfer-resumed                   (transferId: string, resumedFromBytes: number)
//   transfer-progress                 TransferProgress
//   transfer-completed                 (transferId: string, destPath: string)
//   transfer-failed                     (transferId: string, reason: string)
//   transfer-cancelled                   transferId: string
//   transfer-rejected                     transferId: string
//
// Subscribes to all of them and routes each to the matching handler in
// `handlers` (any event without a handler is just ignored). Returns a
// cleanup function — call it once, on unmount, to tear down every
// listener in one shot rather than tracking each unlisten individually.
export async function subscribeToBackendEvents(handlers) {
  const bindings = [
    ["mdns-service-resolved", ([fullname, addr]) => handlers.onServiceResolved?.(fullname, addr)],
    ["mdns-service-removed", (fullname) => handlers.onServiceRemoved?.(fullname)],
    ["join-requested", (peer) => handlers.onJoinRequested?.(peer)],
    ["join-pending", () => handlers.onJoinPending?.()],
    ["joined-network", ([sessionName, hostInfo, roster]) => handlers.onJoinedNetwork?.(sessionName, hostInfo, roster)],
    ["peer-joined", (peer) => handlers.onPeerJoined?.(peer)],
    ["peer-left", (peerId) => handlers.onPeerLeft?.(peerId)],
    ["kicked", (reason) => handlers.onKicked?.(reason)],
    ["disconnected-from-network", () => handlers.onDisconnected?.()],
    ["incoming-file-request", (req) => handlers.onIncomingFileRequest?.(req)],
    ["transfer-started", (t) => handlers.onTransferStarted?.(t)],
    ["transfer-resumed", ([transferId, resumedFromBytes]) => handlers.onTransferResumed?.(transferId, resumedFromBytes)],
    ["transfer-progress", (progress) => handlers.onTransferProgress?.(progress)],
    ["transfer-completed", ([transferId, destPath]) => handlers.onTransferCompleted?.(transferId, destPath)],
    ["transfer-failed", ([transferId, reason]) => handlers.onTransferFailed?.(transferId, reason)],
    ["transfer-cancelled", (transferId) => handlers.onTransferCancelled?.(transferId)],
    ["transfer-rejected", (transferId) => handlers.onTransferRejected?.(transferId)],
  ];

  const unlistens = await Promise.all(
    bindings.map(([name, handler]) => listen(name, (event) => handler(event.payload)))
  );

  return () => unlistens.forEach((unlisten) => unlisten());
}
