use std::path::Path;

pub async fn print_file(path: &Path, printer: &str, copies: u32) -> Result<(), String> {
    let path = path.to_string_lossy().into_owned();
    let printer = printer.to_owned();
    tokio::task::spawn_blocking(move || {
        let printer = printers::get_printer_by_name(&printer)
            .ok_or_else(|| format!("Printer '{printer}' is no longer available"))?;
        let copies = copies.to_string();
        let raw_properties = [("copies", copies.as_str())];
        printer
            .print_file(
                &path,
                printers::common::base::job::PrinterJobOptions {
                    name: None,
                    raw_properties: &raw_properties,
                    converter: printers::common::converters::Converter::None,
                },
            )
            .map(|_| ())
            .map_err(|error| error.message)
    })
    .await
    .map_err(|error| format!("Printer task failed: {error}"))?
}
