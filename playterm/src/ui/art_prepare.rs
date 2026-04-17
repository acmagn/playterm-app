//! Bounded raster prep for `ratatui-image`.
//!
//! `Resize::Scale` in ratatui-image pads the **full** cell×font pixel rectangle; if the source
//! aspect ratio does not match that rectangle, ratatui-image fills the rest with the pad colour
//! (“black bands”). We **center-crop** the cover to the cell aspect ratio first, then scale down
//! inside the 1024 px budget, so the bitmap matches the widget area more closely.

use image::{DynamicImage, ImageBuffer, Rgba, imageops::{self, FilterType}};
use ratatui::layout::Rect;
use ratatui_image::FontSize;

/// Same cap as `kitty_art::render_image` (avoid huge protocol payloads).
pub const MAX_ART_EDGE_PX: u32 = 1024;

/// Home Recently Played strip: encode covers at this multiple of the on-screen pixel budget.
/// The widget still occupies the same `Rect` in cells; ratatui-image scales down for display,
/// but Sixel / halfblocks look much less mushy than fitting straight to `cols×font × rows×font`.
pub const STRIP_ENCODE_SUPERRES: u32 = 2;

/// Pixel size of `inner` in terminal pixels, capped per edge.
pub fn pixel_budget_for_rect(inner: Rect, font: FontSize) -> (u32, u32) {
    let w = (inner.width as u32 * font.0 as u32).min(MAX_ART_EDGE_PX);
    let h = (inner.height as u32 * font.1 as u32).min(MAX_ART_EDGE_PX);
    (w.max(1), h.max(1))
}

/// Scale (up or down) so the image fits inside `max_w × max_h` while preserving aspect ratio.
pub fn fit_image_to_pixel_budget(img: DynamicImage, max_w: u32, max_h: u32) -> DynamicImage {
    let (iw, ih) = (img.width(), img.height());
    if iw == 0 || ih == 0 {
        return img;
    }
    let (tw, th) = fit_inside(iw, ih, max_w, max_h);
    if (tw, th) == (iw, ih) {
        return img;
    }
    img.resize_exact(tw, th, FilterType::Triangle)
}

/// Center-crop `img` to match the aspect ratio of `rect` in terminal pixels (`rect × font`).
pub fn crop_center_to_cell_aspect(img: DynamicImage, rect: Rect, font: FontSize) -> DynamicImage {
    let (iw, ih) = (img.width(), img.height());
    if iw == 0 || ih == 0 || rect.width == 0 || rect.height == 0 {
        return img;
    }
    let cell_w_px = rect.width as u32 * font.0 as u32;
    let cell_h_px = rect.height as u32 * font.1 as u32;
    if cell_w_px == 0 || cell_h_px == 0 {
        return img;
    }
    let tr = cell_w_px as f64 / cell_h_px as f64;
    let ir = iw as f64 / ih as f64;
    if (ir - tr).abs() < 1e-4 {
        return img;
    }
    let (crop_w, crop_h) = if ir > tr {
        let crop_w = (ih as f64 * tr).round() as u32;
        (crop_w.min(iw).max(1), ih)
    } else {
        let crop_h = (iw as f64 / tr).round() as u32;
        (iw, crop_h.min(ih).max(1))
    };
    let x = iw.saturating_sub(crop_w) / 2;
    let y = ih.saturating_sub(crop_h) / 2;
    img.crop_imm(x, y, crop_w, crop_h)
}

/// Center-crop to the cell aspect ratio, then scale into the pixel budget (≤1024 per edge).
pub fn prepare_art_image_for_rect(img: DynamicImage, rect: Rect, font: FontSize) -> DynamicImage {
    let img = crop_center_to_cell_aspect(img, rect, font);
    let (max_w, max_h) = pixel_budget_for_rect(rect, font);
    fit_image_to_pixel_budget(img, max_w, max_h)
}

/// Fit the image **without cropping** into the cell pixel rectangle.
///
/// This preserves the full cover (no top/bottom chop) at the cost of letterboxing.
/// Useful for Sixel terminals where cropping looks especially bad in small strips.
pub fn prepare_art_image_for_rect_contain(img: DynamicImage, rect: Rect, font: FontSize) -> DynamicImage {
    let (max_w, max_h) = pixel_budget_for_rect(rect, font);
    fit_image_to_pixel_budget(img, max_w, max_h)
}

/// Contain-fit into the cell pixel budget, then pad to **exact** `max_w × max_h` with `pad`.
///
/// **ratatui-image contract:** `Picker::new_resize_protocol` builds `ImageSource` with
/// `desired = ceil(bitmap_px / picker.font_size())` in **cells**. The bitmap must therefore use the
/// **same** `FontSize` as the picker, and its pixel size should match the **widget `Rect` × font**
/// (1× budget — not a separate “encode super-res” size), or `desired` will not match the widget
/// and Sixel can leave orphan cells / a black halo.
///
/// Without centered pad, `Resize::Scale` pads again with the picker background (often reads as a
/// second letterbox on Sixel). Matching the panel surface here keeps one consistent matte.
pub fn prepare_art_image_for_rect_contain_centered(
    img: DynamicImage,
    rect: Rect,
    font: FontSize,
    pad: Rgba<u8>,
) -> DynamicImage {
    let fitted = prepare_art_image_for_rect_contain(img, rect, font);
    let (max_w, max_h) = pixel_budget_for_rect(rect, font);
    if fitted.width() == max_w && fitted.height() == max_h {
        return fitted;
    }
    let mut bg: DynamicImage = ImageBuffer::from_pixel(max_w, max_h, pad).into();
    let x = (max_w.saturating_sub(fitted.width())) / 2;
    let y = (max_h.saturating_sub(fitted.height())) / 2;
    imageops::overlay(&mut bg, &fitted, x as i64, y as i64);
    bg
}

/// Like [`prepare_art_image_for_rect`], but targets **2×** the nominal strip pixel size (capped),
/// then **resize_exact** to that rectangle. After `crop_center_to_cell_aspect` the bitmap has the
/// same aspect ratio as the cell, so this is a uniform scale — avoids Sixel paths that
/// width-fit then clip vertically inside the widget.
pub fn prepare_art_image_for_strip(img: DynamicImage, rect: Rect, font: FontSize) -> DynamicImage {
    let img = crop_center_to_cell_aspect(img, rect, font);
    let (bw, bh) = pixel_budget_for_rect(rect, font);
    let max_w = (bw.saturating_mul(STRIP_ENCODE_SUPERRES)).min(MAX_ART_EDGE_PX).max(1);
    let max_h = (bh.saturating_mul(STRIP_ENCODE_SUPERRES)).min(MAX_ART_EDGE_PX).max(1);
    let (iw, ih) = (img.width(), img.height());
    if iw == 0 || ih == 0 {
        return img;
    }
    img.resize_exact(max_w, max_h, FilterType::Triangle)
}

fn fit_inside(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let wratio = max_w as f64 / w as f64;
    let hratio = max_h as f64 / h as f64;
    let ratio = f64::min(wratio, hratio);
    let nw = ((w as f64 * ratio).round() as u32).max(1);
    let nh = ((h as f64 * ratio).round() as u32).max(1);
    (nw, nh)
}

/// FNV-1a 64-bit digest of raw image bytes.
///
/// Used for Now Playing cache keys so consecutive tracks with different `cover_id` but identical
/// pixels do not trigger re-encode / re-transmit.
pub fn art_bytes_fingerprint(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 14695981039346656037;
    const PRIME: u64 = 1099511628211;
    let mut h = OFFSET;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(PRIME);
    }
    h
}
