//! Local QR-code generation for links.
//!
//! Linkly's API exposes no QR image endpoint, so we render QR codes ourselves
//! from each link's short URL (`full_url`) and write them as PNG files. This
//! works for a single link or as a batch over a whole workspace.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::Luma;
use qrcode::QrCode;

/// Directory (relative to the working directory) QR PNGs are written to.
pub fn output_dir() -> PathBuf {
    PathBuf::from("linkly-qr")
}

/// Turn a slug/name into a filesystem-safe fragment.
fn sanitize(s: &str) -> String {
    let cleaned: String = s
        .trim_start_matches('/')
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    cleaned.trim_matches('-').to_string()
}

/// Build a PNG filename for a link, preferring its slug, then name, then id.
pub fn file_name(id: Option<i64>, slug: Option<&str>, name: Option<&str>) -> String {
    let base = slug
        .map(sanitize)
        .filter(|s| !s.is_empty())
        .or_else(|| name.map(sanitize).filter(|s| !s.is_empty()));
    match (id, base) {
        (Some(id), Some(b)) => format!("{id}-{b}.png"),
        (Some(id), None) => format!("link-{id}.png"),
        (None, Some(b)) => format!("{b}.png"),
        (None, None) => "link.png".to_string(),
    }
}

/// Render `url` as a QR code and write it to `path` (creating parent dirs).
pub fn write_qr(url: &str, path: &Path) -> Result<()> {
    let code = QrCode::new(url.as_bytes()).context("encoding QR code")?;
    let img = code
        .render::<Luma<u8>>()
        .min_dimensions(320, 320)
        .quiet_zone(true)
        .build();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    img.save(path)
        .with_context(|| format!("saving {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_are_safe_and_prefixed() {
        assert_eq!(file_name(Some(30), Some("/promo"), None), "30-promo.png");
        assert_eq!(
            file_name(Some(7), Some("/sale?x=1"), None),
            "7-sale-x-1.png"
        );
        assert_eq!(file_name(Some(9), None, Some("My Link")), "9-My-Link.png");
        assert_eq!(file_name(Some(5), Some("/"), None), "link-5.png");
        assert_eq!(file_name(None, None, None), "link.png");
    }

    #[test]
    fn writes_a_png() {
        let dir = std::env::temp_dir().join(format!("linkly-qr-test-{}", std::process::id()));
        let path = dir.join("t.png");
        write_qr("https://example.com/promo", &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        // PNG magic number.
        assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
