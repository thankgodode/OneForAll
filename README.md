# Tauri + React

This template should help get you started developing with Tauri and React in Vite.

## Printing

Print jobs are sent to a connected peer's installed printer queue. The receiver must approve each print job before the document is transferred and printed. The temporary received file and any converted document are removed after printing. Print jobs have their own status list and do not appear in file-transfer progress.

On Linux and macOS, `printers` submits PDF files through CUPS. JPEG, PNG, BMP, GIF, TIFF, and WebP images are converted to PDF in the app; DOCX, XLSX, and PPTX are converted with the pure-Rust `office2pdf` crate. Windows printing uses Ghostscript's `mswinpr2` device and requires Ghostscript on the receiving device; other platforms use their installed CUPS printer queue. Unsupported office formats fail with an explanatory status instead of sending document bytes to the printer as raw data.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
