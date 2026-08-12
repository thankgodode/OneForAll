import { LayoutGrid, Send, ArrowUpRight, Printer, Wifi, Settings } from "lucide-react";

// Single source of truth for sidebar nav + page titles.
// Add a new view by adding one entry here — Sidebar and App pick it up automatically.
export const NAV = [
  { id: "dashboard", label: "Dashboard", icon: LayoutGrid },
  { id: "send", label: "Send files", icon: Send },
  { id: "transfers", label: "Transfers", icon: ArrowUpRight },
  { id: "print", label: "Print", icon: Printer },
  { id: "network", label: "Network", icon: Wifi },
  { id: "settings", label: "Profile & Settings", icon: Settings },
];

export const VIEW_TITLES = {
  dashboard: ["Dashboard", "Everything happening on your network, at a glance"],
  send: ["Send files", "Choose files and pick who receives them"],
  transfers: ["Transfers", "Track active, completed, and failed transfers"],
  print: ["Print", "Send documents to a printer connected on the LAN"],
  network: ["Network", "Host a new session or join one nearby"],
  settings: ["Profile & Settings", "Your identity, preferences, and device info"],
};
