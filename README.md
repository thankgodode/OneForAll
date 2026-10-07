# Tauri + React

This template should help get you started developing with Tauri and React in Vite.

## Printing

Print jobs are sent to a connected peer's installed printer queue using the Rust `printers` crate. The receiver must approve each print job before the document is transferred and printed. The temporary received file is removed after it is queued, and the requested copy count is sent to the printer. Print jobs have their own status list and do not appear in file-transfer progress. The crate uses Windows Print Spooler on Windows and CUPS on Unix-like systems.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
