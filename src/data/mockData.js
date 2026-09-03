// Mock data + static lookup tables. STATUS_META and JOB_STATUS_META are
// real lookup tables still used throughout the UI. initialPrintJobs stays
// mock seed data since there's no backend support for printing yet —
// devices/transfers are no longer mocked here; they come from real
// backend state via App.jsx (see src/api/).

export const STATUS_META = {
  online:       { label: "Online",       color: "var(--teal)" },
  transferring: { label: "Transferring", color: "var(--blue)" },
  busy:         { label: "Busy",         color: "var(--amber)" },
  offline:      { label: "Offline",      color: "var(--gray)" },
};

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
