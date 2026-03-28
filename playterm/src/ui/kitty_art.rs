//! Kitty terminal graphics protocol helpers.
//!
//! Provides detection, rendering, and clearing of album art using the
//! [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/).
//!
//! Images are transmitted with `a=T` (transmit-and-display), `f=32` (RGBA8),
//! `o=z` (zlib-compressed), and positioned via a preceding cursor-move escape.

use std::io::{self, Write};

use anyhow::Result;
use ratatui::layout::Rect;

// ── Detection ──────────────────────────────────────────────────────────────────

/// Returns `true` if the running terminal supports the Kitty graphics protocol.
///
/// Checks environment variables set by known-compatible terminals:
/// - `KITTY_WINDOW_ID` (native Kitty)
/// - `TERM=xterm-kitty`
/// - `TERM_PROGRAM=WezTerm`
pub fn detect_kitty_support() -> bool {
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return true;
    }
    if let Ok(term) = std::env::var("TERM") {
        if term == "xterm-kitty" {
            return true;
        }
    }
    if let Ok(prog) = std::env::var("TERM_PROGRAM") {
        if prog == "WezTerm" {
            return true;
        }
    }
    false
}

// ── Rendering ─────────────────────────────────────────────────────────────────

/// Render image `bytes` (JPEG/PNG/etc.) into `area` using the Kitty graphics protocol.
///
/// `area` is the full widget rect (including borders); the image is placed in the
/// inner area (1-cell border inset on all sides).  Writes directly to stdout.
pub fn render_image(bytes: &[u8], area: Rect) -> Result<()> {
    use base64::Engine;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;

    let inner_x = area.x + 1;
    let inner_y = area.y + 1;
    let inner_w = area.width.saturating_sub(2);
    let inner_h = area.height.saturating_sub(2);
    if inner_w == 0 || inner_h == 0 {
        return Ok(());
    }

    // Decode image from raw bytes.
    let img = image::load_from_memory(bytes)?;

    // Resize to fit the inner area.  We estimate 10 px per column, 20 px per row
    // (a reasonable approximation for most terminals).  Cap at 1024 to avoid
    // transferring enormous payloads on very large terminals.
    let px_w = (inner_w as u32 * 10).min(1024);
    let px_h = (inner_h as u32 * 20).min(1024);
    let img = img.resize(px_w, px_h, image::imageops::FilterType::Lanczos3);
    let img_rgba = img.to_rgba8();
    let (w, h) = img_rgba.dimensions();
    let raw = img_rgba.into_raw();

    // Zlib-compress the raw RGBA bytes.
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&raw)?;
    let compressed = enc.finish()?;

    // Base64-encode.
    let b64 = base64::engine::general_purpose::STANDARD.encode(&compressed);

    // Write to stdout.
    let mut out = io::stdout().lock();

    // Move cursor to the inner-area top-left (terminal coords are 1-based).
    write!(out, "\x1b[{};{}H", inner_y + 1, inner_x + 1)?;

    // Transmit the image in ≤4096-char chunks.
    const CHUNK: usize = 4096;
    let chunks: Vec<&[u8]> = b64.as_bytes().chunks(CHUNK).collect();
    let n = chunks.len();

    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == n - 1;
        let m = if is_last { 0u8 } else { 1u8 };
        // SAFETY: b64 is ASCII, so each chunk is valid UTF-8.
        let chunk_str = unsafe { std::str::from_utf8_unchecked(chunk) };
        if i == 0 {
            // First chunk: include all control parameters.
            write!(
                out,
                "\x1b_Ga=T,f=32,s={w},v={h},c={inner_w},r={inner_h},o=z,m={m},q=2;{chunk_str}\x1b\\"
            )?;
        } else {
            write!(out, "\x1b_Gm={m};{chunk_str}\x1b\\")?;
        }
    }

    out.flush()?;
    Ok(())
}

// ── Clearing ──────────────────────────────────────────────────────────────────

/// Delete all Kitty images currently displayed in the terminal.
pub fn clear_image() -> Result<()> {
    let mut out = io::stdout().lock();
    write!(out, "\x1b_Ga=d,d=A,q=2\x1b\\")?;
    out.flush()?;
    Ok(())
}
