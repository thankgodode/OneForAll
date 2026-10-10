#[cfg(target_os = "windows")]
use std::process::Command;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub async fn print_file(path: &Path, printer: &str, copies: u32) -> Result<(), String> {
    let path = path.to_path_buf();
    let printer = printer.to_owned();
    tokio::task::spawn_blocking(move || print_file_blocking(&path, &printer, copies))
        .await
        .map_err(|error| format!("Printer task failed: {error}"))?
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrintableFormat {
    Pdf,
    Image,
    Office,
    PostScript,
}

fn print_file_blocking(path: &Path, printer_name: &str, copies: u32) -> Result<(), String> {
    let format = classify_format(path)?;
    let conversion_dir = if format == PrintableFormat::Pdf {
        None
    } else {
        Some(create_conversion_dir()?)
    };

    let result = (|| {
        let printable_path = match (format, conversion_dir.as_deref()) {
            (PrintableFormat::Pdf, _) => path.to_path_buf(),
            (PrintableFormat::Image, Some(dir)) => {
                let output = dir.join("print.pdf");
                convert_image_to_pdf(path, &output)?;
                output
            }
            (PrintableFormat::Office, Some(dir)) => convert_with_office2pdf(path, dir)?,
            _ => return Err("Could not prepare the document for printing".into()),
        };

        print_pdf(&printable_path, printer_name, copies)
    })();

    let cleanup = match conversion_dir {
        Some(dir) => fs::remove_dir_all(dir),
        None => Ok(()),
    };
    match (result, cleanup) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(format!(
            "Print submitted, but temporary conversion files could not be removed: {error}"
        )),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn classify_format(path: &Path) -> Result<PrintableFormat, String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "pdf" => Ok(PrintableFormat::Pdf),
        "jpg" | "jpeg" | "png" | "bmp" | "gif" | "tif" | "tiff" | "webp" => {
            Ok(PrintableFormat::Image)
        }
        "ps" | "eps" => Ok(PrintableFormat::PostScript),
        "doc" | "docx" | "docm" | "odt" | "rtf" | "txt" | "xls" | "xlsx" | "xlsm"
        | "ods" | "csv" | "ppt" | "pptx" | "pptm" | "odp" | "html" | "htm" | "svg" => {
            Ok(PrintableFormat::Office)
        }
        _ => Err(format!(
            "Unsupported print format '.{}'; supported formats include PDF, common images, office documents, and PostScript",
            if extension.is_empty() { "unknown" } else { &extension }
        )),
    }
}

fn create_conversion_dir() -> Result<PathBuf, String> {
    for _ in 0..3 {
        let path = env::temp_dir().join(format!("oneforall-print-{}", uuid::Uuid::new_v4()));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "Couldn't create a temporary print directory: {error}"
                ))
            }
        }
    }
    Err("Couldn't create a unique temporary print directory".into())
}

fn convert_image_to_pdf(input: &Path, output: &Path) -> Result<(), String> {
    let bytes = fs::read(input).map_err(|error| format!("Couldn't read image: {error}"))?;
    let pdf = image_pdf_bytes(&bytes)?;
    fs::write(output, pdf).map_err(|error| format!("Couldn't save converted image: {error}"))
}

fn image_pdf_bytes(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let image = printpdf::RawImage::decode_from_bytes(bytes, &mut Vec::new())
        .map_err(|error| format!("Couldn't decode image: {error}"))?;
    raw_image_pdf_bytes(image)
}

fn raw_image_pdf_bytes(image: printpdf::RawImage) -> Result<Vec<u8>, String> {
    use printpdf::{Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Pt, XObjectTransform};

    let landscape = image.width > image.height;
    let (page_width_mm, page_height_mm) = if landscape {
        (297.0, 210.0)
    } else {
        (210.0, 297.0)
    };

    const DPI: f32 = 300.0;
    const MARGIN_PT: f32 = 18.0;
    let page_width_pt = page_width_mm * 72.0 / 25.4;
    let page_height_pt = page_height_mm * 72.0 / 25.4;
    let image_width_pt = image.width as f32 * 72.0 / DPI;
    let image_height_pt = image.height as f32 * 72.0 / DPI;
    let scale = ((page_width_pt - 2.0 * MARGIN_PT) / image_width_pt)
        .min((page_height_pt - 2.0 * MARGIN_PT) / image_height_pt);
    let x = (page_width_pt - image_width_pt * scale) / 2.0;
    let y = (page_height_pt - image_height_pt * scale) / 2.0;

    let mut document = PdfDocument::new("OneForAll print job");
    let image_id = document.add_image(&image);
    let page = PdfPage::new(
        Mm(page_width_mm),
        Mm(page_height_mm),
        vec![Op::UseXobject {
            id: image_id,
            transform: XObjectTransform {
                translate_x: Some(Pt(x)),
                translate_y: Some(Pt(y)),
                scale_x: Some(scale),
                scale_y: Some(scale),
                dpi: Some(DPI),
                rotate: None,
            },
        }],
    );

    Ok(document
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut Vec::new()))
}

fn convert_with_office2pdf(input: &Path, output_dir: &Path) -> Result<PathBuf, String> {
    let format = input
        .extension()
        .and_then(|extension| extension.to_str())
        .and_then(office2pdf::config::Format::from_extension)
        .ok_or_else(|| {
            format!(
                "office2pdf supports DOCX, XLSX, and PPTX, not '{}'",
                input
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or("unknown")
            )
        })?;
    let result = office2pdf::convert_bytes(
        &fs::read(input).map_err(|error| format!("Couldn't read office document: {error}"))?,
        format,
        &office2pdf::config::ConvertOptions::default(),
    )
    .map_err(|error| format!("Office document conversion failed: {error}"))?;
    let output = output_dir.join("print.pdf");
    fs::write(&output, result.pdf)
        .map_err(|error| format!("Couldn't save converted office document: {error}"))?;
    Ok(output)
}

#[cfg(target_os = "windows")]
fn print_pdf(path: &Path, printer_name: &str, copies: u32) -> Result<(), String> {
    let gs_exe = find_program(&ghostscript_programs(), &[])
        .ok_or_else(|| "Ghostscript is required to print on Windows".to_string())?;

    let mut cmd = Command::new(gs_exe);

    cmd.args(["-dBATCH", "-dNOPAUSE", "-dQUIET", "-dNoCancel"])
        .arg("-sDEVICE=mswinpr2")
        .args(["-dFIXEDMEDIA", "-dPDFFitPage"])
        .arg(format!("-dNumCopies={}", copies));

    // Rust quotes the whole argument for you, so printer names with spaces are fine.
    cmd.arg(format!("-sOutputFile=%printer%{}", printer_name))
        .arg(path);

    // Don't flash a console window
    // cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

    let out = cmd
        .output()
        .map_err(|e| format!("failed to launch gs: {e}"))?;

    if out.status.success() {
        Ok(())
    } else {
        return Err(format!(
            "ghostscript exited with {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
}

#[cfg(not(target_os = "windows"))]
fn print_pdf(path: &Path, printer_name: &str, copies: u32) -> Result<(), String> {
    let printer = printers::get_printer_by_name(printer_name)
        .ok_or_else(|| format!("Printer '{printer_name}' is no longer available"))?;
    let path = path.to_string_lossy().into_owned();
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
}

#[cfg(target_os = "windows")]
fn find_program(names: &[&str], extra_paths: &[PathBuf]) -> Option<PathBuf> {
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            for name in names {
                let candidate = directory.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    extra_paths.iter().find(|path| path.is_file()).cloned()
}

#[cfg(target_os = "windows")]
fn ghostscript_programs() -> [&'static str; 2] {
    ["gswin64c.exe", "gswin32c.exe"]
}

#[cfg(test)]
mod tests {
    use super::{classify_format, raw_image_pdf_bytes, PrintableFormat};
    use printpdf::{RawImage, RawImageData, RawImageFormat};
    use std::path::Path;

    #[test]
    fn classifies_printable_document_types_case_insensitively() {
        assert_eq!(
            classify_format(Path::new("report.DOCX")).unwrap(),
            PrintableFormat::Office
        );
        assert_eq!(
            classify_format(Path::new("photo.JPEG")).unwrap(),
            PrintableFormat::Image
        );
        assert_eq!(
            classify_format(Path::new("animation.gif")).unwrap(),
            PrintableFormat::Image
        );
        assert_eq!(
            classify_format(Path::new("scan.tiff")).unwrap(),
            PrintableFormat::Image
        );
        assert_eq!(
            classify_format(Path::new("scan.PDF")).unwrap(),
            PrintableFormat::Pdf
        );
        assert_eq!(
            classify_format(Path::new("document.ps")).unwrap(),
            PrintableFormat::PostScript
        );
        assert!(classify_format(Path::new("archive.zip")).is_err());
    }

    #[test]
    fn converts_image_bytes_to_pdf() {
        let image = RawImage {
            pixels: RawImageData::U8(vec![255, 0, 0]),
            width: 1,
            height: 1,
            data_format: RawImageFormat::RGB8,
            tag: Vec::new(),
        };
        let pdf = raw_image_pdf_bytes(image).unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
    }
}
