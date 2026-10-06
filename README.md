# Tauri + React

This template should help get you started developing with Tauri and React in Vite.

## Printing

Print jobs are sent to a connected peer's installed printer queue. The receiver must approve each print job before the document is transferred and printed. On Windows, installed queues are read from the Windows printer service; on macOS and Linux, CUPS (`lpstat` and `lp`) is used. Printing relies on the receiving system's registered print handler for the selected document type.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
