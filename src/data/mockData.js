// Static lookup tables used by the UI.

export const STATUS_META = {
  online:       { label: "Online",       color: "var(--teal)" },
  transferring: { label: "Transferring", color: "var(--blue)" },
  busy:         { label: "Busy",         color: "var(--amber)" },
  offline:      { label: "Offline",      color: "var(--gray)" },
};

export const JOB_STATUS_META = {
  pending:   { color: "var(--amber)", label: "Awaiting approval" },
  printing:  { color: "var(--blue)",  label: "Printing" },
  sent:      { color: "var(--blue)",  label: "Sent to device" },
  completed: { color: "var(--teal)",  label: "Completed" },
  rejected:  { color: "var(--red)",   label: "Rejected" },
  failed:    { color: "var(--red)",   label: "Failed" },
};
