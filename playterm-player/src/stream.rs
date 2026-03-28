//! HTTP → temp-file download for Phase 1 audio pipeline.
//!
//! Uses `reqwest::blocking` so it can be called from the player's `std::thread`
//! without requiring a tokio runtime.  Phase 2 will replace this with a ring
//! buffer approach.

use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};

/// Download `url` to a temporary file and return the path.
///
/// The file is created in the system temp directory with a `.audio` suffix.
/// The caller is responsible for deleting it when done.
pub fn download_to_tempfile(url: &str) -> Result<PathBuf> {
    let mut response = reqwest::blocking::get(url)
        .context("HTTP request failed")?;

    // Build a temp path unique to this download.
    let pid = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("playterm-{pid}-{ts}.audio"));

    let mut file = std::fs::File::create(&path)
        .with_context(|| format!("could not create temp file at {}", path.display()))?;

    std::io::copy(&mut response, &mut file).context("download failed")?;
    file.flush()?;

    Ok(path)
}
