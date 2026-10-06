use serde::{Deserialize, Serialize};

use printers::{
    common::base::job::PrinterJobOptions, get_default_printer, get_printer_by_name, get_printers,
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

/// Returns a stable ID based on the printer name so peers can address the
/// same installed printer across refreshes and application restarts.
fn printer_id(name: &str) -> String {
    let hash = name
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("printer-{hash:016x}")
}

/// Enumerate printers installed on this computer.
pub fn list_local_printers() -> Vec<PrinterInfo> {
    let default_name = get_default_printer().map(|printer| printer.name.to_string());

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
pub fn print_file(printer_name: &str, file_path: &str, copies: u32) -> Result<(), String> {
    let printer = get_printer_by_name(printer_name)
        .ok_or_else(|| format!("Printer '{}' was not found", printer_name))?;

    let copies = copies.max(1).to_string();

    let options = PrinterJobOptions {
        name: None,
        raw_properties: &[("copies", copies.as_str())],
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

#[cfg(test)]
mod tests {
    use super::printer_id;

    #[test]
    fn printer_id_is_stable_and_distinguishes_names() {
        assert_eq!(printer_id("Office Printer"), printer_id("Office Printer"));
        assert_ne!(printer_id("Office Printer"), printer_id("Photo Printer"));
    }
}
