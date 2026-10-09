use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
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
            (PrintableFormat::Office, Some(dir)) => convert_with_libreoffice(path, dir)?,
            (PrintableFormat::PostScript, Some(dir)) => convert_with_ghostscript(path, dir)?,
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

fn convert_with_libreoffice(input: &Path, output_dir: &Path) -> Result<PathBuf, String> {
    let executable = find_program(
        &["soffice", "libreoffice"],
        &libreoffice_install_paths(),
    )
    .ok_or_else(|| {
        "This document format requires LibreOffice on the receiving device. Install LibreOffice and try again.".to_string()
    })?;
    let profile = output_dir.join("libreoffice-profile");
    let profile_url = file_url(&profile);
    let output = Command::new(&executable)
        .args([
            "--headless",
            "--nologo",
            "--nodefault",
            "--nolockcheck",
            "--norestore",
        ])
        .arg(format!("-env:UserInstallation={profile_url}"))
        .args(["--convert-to", "pdf", "--outdir"])
        .arg(output_dir)
        .arg(input)
        .output()
        .map_err(|error| format!("Couldn't start LibreOffice: {error}"))?;
    if !output.status.success() {
        return Err(command_error("LibreOffice conversion", &output));
    }

    let output_path = output_dir.join(
        input
            .file_stem()
            .ok_or_else(|| "The document filename has no name".to_string())?
            .to_string_lossy()
            .to_string()
            + ".pdf",
    );
    if !output_path.is_file() {
        return Err("LibreOffice completed without creating a PDF".into());
    }
    Ok(output_path)
}

fn convert_with_ghostscript(input: &Path, output_dir: &Path) -> Result<PathBuf, String> {
    let executable = find_program(&ghostscript_programs(), &[])
        .ok_or_else(|| {
            "PostScript printing requires Ghostscript on the receiving device. Install Ghostscript and try again.".to_string()
        })?;
    let output_path = output_dir.join("print.pdf");
    let output = Command::new(executable)
        .args(["-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=pdfwrite"])
        .arg(format!("-sOutputFile={}", output_path.display()))
        .arg(input)
        .output()
        .map_err(|error| format!("Couldn't start Ghostscript: {error}"))?;
    if !output.status.success() {
        return Err(command_error("Ghostscript conversion", &output));
    }
    if !output_path.is_file() {
        return Err("Ghostscript completed without creating a PDF".into());
    }
    Ok(output_path)
}

fn print_pdf(path: &Path, printer_name: &str, copies: u32) -> Result<(), String> {
    let printer = printers::get_printer_by_name(printer_name)
        .ok_or_else(|| format!("Printer '{printer_name}' is no longer available"))?;

    #[cfg(target_os = "windows")]
    {
        let executable = find_program(&["SumatraPDF.exe", "SumatraPDF"], &sumatra_install_paths())
            .ok_or_else(|| {
                "PDF printing on Windows requires SumatraPDF on the receiving device. Install SumatraPDF and try again.".to_string()
            })?;
        let output = Command::new(executable)
            .args(["-print-to", printer.system_name.as_str(), "-print-settings"])
            .arg(format!("{copies}x"))
            .arg("-silent")
            .arg(path)
            .output()
            .map_err(|error| format!("Couldn't start SumatraPDF: {error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(command_error("SumatraPDF printing", &output))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
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
}

fn command_error(operation: &str, output: &std::process::Output) -> String {
    let details = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let code = output
        .status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "terminated by signal".into());
    if details.is_empty() {
        format!("{operation} failed (exit code {code})")
    } else {
        format!("{operation} failed (exit code {code}): {details}")
    }
}

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

fn libreoffice_install_paths() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return [
            env::var_os("ProgramFiles"),
            env::var_os("ProgramFiles(x86)"),
        ]
        .into_iter()
        .flatten()
        .map(|root| PathBuf::from(root).join("LibreOffice/program/soffice.exe"))
        .collect();
    }
    #[cfg(target_os = "macos")]
    {
        vec![PathBuf::from(
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
        )]
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "windows")]
fn sumatra_install_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for root in [env::var_os("ProgramFiles"), env::var_os("LOCALAPPDATA")]
        .into_iter()
        .flatten()
    {
        paths.push(PathBuf::from(root).join("SumatraPDF/SumatraPDF.exe"));
    }
    paths
}

#[cfg(target_os = "windows")]
fn ghostscript_programs() -> [&'static str; 2] {
    ["gswin64c.exe", "gswin32c.exe"]
}

#[cfg(not(target_os = "windows"))]
fn ghostscript_programs() -> [&'static str; 1] {
    ["gs"]
}

fn file_url(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    let path = path
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect::<String>();
    if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_format, file_url, raw_image_pdf_bytes, PrintableFormat};
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

    #[test]
    fn creates_file_urls_for_paths_with_spaces() {
        assert_eq!(file_url(Path::new("/tmp/my file")), "file:///tmp/my%20file");
    }
}
