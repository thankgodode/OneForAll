# OneForAll

OneForAll is a desktop app for sharing files and sending print jobs between devices on the same network. It helps you move files between peers and print documents using a printer connected to another device.

> **Beta:** OneForAll is under active testing. Printer compatibility and supported formats may vary by operating system, drivers, and installed dependencies.

## Features

- Discover and join peer networks to share files between devices.
- Send print jobs to a peer’s installed printer.
- Set the number of copies and configure incoming print-job confirmation.
- Track print jobs separately from file-transfer progress.
- Receive notification toasts and optional sound alerts.
- Convert common image and office document formats for printing.

## Printing requirements

The receiving device must be running OneForAll and have a printer installed. On Windows, printing requires Ghostscript. On Linux and macOS, printer submission uses the system’s CUPS printer queue.

Image formats supported for conversion include JPEG, PNG, BMP, GIF, TIFF, and WebP. DOCX, XLSX, and PPTX files are converted using `office2pdf`.

## Development

### Requirements

- Node.js and npm
- Rust and Cargo
- The platform prerequisites for [Tauri 2](https://v2.tauri.app/start/prerequisites/)

### Run locally

```bash
npm install
npm run tauri dev
