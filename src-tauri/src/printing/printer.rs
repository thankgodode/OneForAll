use serde::{Deserialize, Serialize};

use printers::{
    common::base::job::PrinterJobOptions,
    get_default_printer,
    get_printer_by_name,
    get_printers,
};

/// Information about a printer that can safely be sent over the network.
///
/// This intentionally contains only information other peers need to display
/// and identify the printer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrinterInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// Returns a stable-ish ID based on the printer name.
///
/// For the first implementation this is enough. We don't want the printer
/// name itself to be our protocol identifier because names can contain
/// characters that are inconvenient to work with and can potentially change.
fn printer_id(name: &str) -> String {
    format!(
        "printer-{}",
        uuid::Uuid::new_v4()
    )
}

/// Enumerate printers installed on this computer.
pub fn list_local_printers() -> Vec<PrinterInfo> {
    let default_name = get_default_printer()
        .map(|printer| printer.name.to_string());

    get_printers()
        .into_iter()
        .map(|printer| {
            let name = printer.name.to_string();

            PrinterInfo {
                id: printer_id(&name),
                name: name.clone(),
                is_default: default_name.as_deref() == Some(name.as_str()),
            }
        })
        .collect()
}

/// Print a file using one of this computer's local printers.
///
/// The file must already exist locally.
pub fn print_file(
    printer_name: &str,
    file_path: &str,
    copies: u32,
) -> Result<(), String> {
    let printer = get_printer_by_name(printer_name)
        .ok_or_else(|| format!("Printer '{}' was not found", printer_name))?;

    let copies = copies.max(1).to_string();

    let options = PrinterJobOptions {
        name: None,
        raw_properties: &[
            ("copies", copies.as_str()),
        ],
        converter: printers::common::converters::Converter::None,
    };

    printer
        .print_file(file_path, options)
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub fn get_local_printers() -> Vec<PrinterInfo> {
    list_local_printers()
}