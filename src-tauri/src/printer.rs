use std::path::Path;

pub async fn print_file(path: &Path, printer: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let output = tokio::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "param($file, $printer); Start-Process -FilePath $file -Verb PrintTo -ArgumentList $printer",
        ])
        .arg(path)
        .arg(printer)
        .output()
        .await;

    #[cfg(not(target_os = "windows"))]
    let output = tokio::process::Command::new("lp")
        .arg("-d")
        .arg(printer)
        .arg(path)
        .output()
        .await;

    let output = output.map_err(|error| format!("Couldn't start the system print service: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(if detail.is_empty() {
            format!("The system print service exited with {}", output.status)
        } else {
            detail
        })
    }
}
