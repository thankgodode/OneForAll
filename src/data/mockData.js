// Mock data + static lookup tables. Swap these for real data sources
// (device discovery, transfer queue, print queue) without touching
// any component — every view reads from props, not from this file directly.

export const STATUS_META = {
  online:       { label: "Online",       color: "var(--teal)" },
  transferring: { label: "Transferring", color: "var(--blue)" },
  busy:         { label: "Busy",         color: "var(--amber)" },
  offline:      { label: "Offline",      color: "var(--gray)" },
};

export const initialDevices = [
  { id: "you", name: "You", device: "This device", status: "online", isSelf: true, hasPrinter: true, printerName: "Home LaserJet", icon: "laptop", color: "#3FDCC4" },
  { id: "d1", name: "Ada", device: "Ada — Workstation", status: "online", hasPrinter: true, printerName: "HP LaserJet M404", icon: "monitor", color: "#5B9FE0" },
  { id: "d2", name: "Kernel-Box", device: "Kernel-Box (Linux)", status: "transferring", hasPrinter: false, icon: "server", color: "#E8A63D" },
  { id: "d3", name: "Prima", device: "Prima's Studio", status: "online", hasPrinter: true, printerName: "Canon PIXMA TS8320", icon: "laptop", color: "#B383E8" },
  { id: "d4", name: "Rex", device: "Rex — Old ThinkPad", status: "busy", hasPrinter: false, icon: "laptop", color: "#E86B95" },
  { id: "d5", name: "Nadia", device: "Nadia's MacBook", status: "offline", hasPrinter: false, icon: "laptop", color: "#5D6E71" },
];

export const initialTransfers = [
  { id: "t1", file: "quarterly-report.pdf", size: "4.2 MB", target: "Ada", direction: "send", status: "active", progress: 62, speed: "3.1 MB/s", eta: "2s" },
  { id: "t2", file: "site-assets.zip", size: "128 MB", target: "Prima", direction: "send", status: "active", progress: 18, speed: "6.4 MB/s", eta: "18s" },
  { id: "t3", file: "vacation-photos.zip", size: "89 MB", target: "You", direction: "receive", status: "completed", progress: 100 },
  { id: "t4", file: "invoice-0432.docx", size: "220 KB", target: "Rex", direction: "send", status: "failed", progress: 34, error: "Connection lost" },
  { id: "t5", file: "dataset.csv", size: "12.8 MB", target: "You", direction: "receive", status: "completed", progress: 100 },
];

export const initialPrintJobs = [
  { id: "p1", file: "contract-signed.pdf", target: "Ada", printer: "HP LaserJet M404", status: "printing", pages: 4 },
  { id: "p2", file: "boarding-pass.pdf", target: "Prima", printer: "Canon PIXMA TS8320", status: "pending", pages: 1 },
  { id: "p3", file: "recipe-card.pdf", target: "Ada", printer: "HP LaserJet M404", status: "completed", pages: 2 },
  { id: "p4", file: "flyer-draft.pdf", target: "Prima", printer: "Canon PIXMA TS8320", status: "rejected", pages: 6 },
];

export const JOB_STATUS_META = {
  pending:   { color: "var(--amber)", label: "Awaiting approval" },
  printing:  { color: "var(--blue)",  label: "Printing" },
  completed: { color: "var(--teal)",  label: "Completed" },
  rejected:  { color: "var(--red)",   label: "Rejected" },
};
