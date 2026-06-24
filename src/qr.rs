//! Local QR-code generation for links.
//!
//! Linkly's API exposes no QR image endpoint, so we render QR codes ourselves
//! from each link's short URL (`full_url`). Output format, size and colours are
//! configurable via [`QrSettings`]; files are written under
//! `linkly-qr/<workspace-id>/`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use image::Rgb;
use qrcode::render::svg;
use qrcode::QrCode;
use serde::{Deserialize, Serialize};

/// Output image format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QrFormat {
    Png,
    Svg,
    Jpeg,
}

impl QrFormat {
    pub const ALL: [QrFormat; 3] = [QrFormat::Png, QrFormat::Svg, QrFormat::Jpeg];

    pub fn ext(self) -> &'static str {
        match self {
            QrFormat::Png => "png",
            QrFormat::Svg => "svg",
            QrFormat::Jpeg => "jpg",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            QrFormat::Png => "PNG",
            QrFormat::Svg => "SVG",
            QrFormat::Jpeg => "JPEG",
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|f| *f == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let i = Self::ALL.iter().position(|f| *f == self).unwrap_or(0);
        Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// User-configurable QR export options (persisted).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrSettings {
    pub format: QrFormat,
    /// Minimum image dimension in pixels (clamped on edit).
    pub size: u32,
    /// Foreground (dark) colour as `#rrggbb`.
    pub fg: String,
    /// Background (light) colour as `#rrggbb`.
    pub bg: String,
}

impl Default for QrSettings {
    fn default() -> Self {
        Self {
            format: QrFormat::Png,
            size: 512,
            fg: "#000000".to_string(),
            bg: "#ffffff".to_string(),
        }
    }
}

/// Directory QR files for a workspace are written to: `linkly-qr/<id>/`.
pub fn output_dir(workspace_id: i64) -> PathBuf {
    PathBuf::from("linkly-qr").join(workspace_id.to_string())
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

/// Build a filename for a link, preferring its slug, then name, then id, with
/// the extension for `format`.
pub fn file_name(id: Option<i64>, slug: Option<&str>, name: Option<&str>, format: QrFormat) -> String {
    let ext = format.ext();
    let base = slug
        .map(sanitize)
        .filter(|s| !s.is_empty())
        .or_else(|| name.map(sanitize).filter(|s| !s.is_empty()));
    match (id, base) {
        (Some(id), Some(b)) => format!("{id}-{b}.{ext}"),
        (Some(id), None) => format!("link-{id}.{ext}"),
        (None, Some(b)) => format!("{b}.{ext}"),
        (None, None) => format!("link.{ext}"),
    }
}

/// Parse `#rrggbb` (or `rrggbb`) into RGB bytes.
fn parse_hex(s: &str) -> Result<[u8; 3]> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 {
        bail!("colour must be #rrggbb, got '{s}'");
    }
    let r = u8::from_str_radix(&h[0..2], 16).context("bad red")?;
    let g = u8::from_str_radix(&h[2..4], 16).context("bad green")?;
    let b = u8::from_str_radix(&h[4..6], 16).context("bad blue")?;
    Ok([r, g, b])
}

/// Validate a colour and return it normalised as `#rrggbb`, or `None` if invalid.
pub fn normalize_color(s: &str) -> Option<String> {
    parse_hex(s)
        .ok()
        .map(|[r, g, b]| format!("#{r:02x}{g:02x}{b:02x}"))
}

/// Render `url` as a QR code and write it to `path` per `settings`.
pub fn write_qr(url: &str, path: &Path, settings: &QrSettings) -> Result<()> {
    let code = QrCode::new(url.as_bytes()).context("encoding QR code")?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }

    match settings.format {
        QrFormat::Svg => {
            let svg_str = code
                .render::<svg::Color>()
                .min_dimensions(settings.size, settings.size)
                .dark_color(svg::Color(&settings.fg))
                .light_color(svg::Color(&settings.bg))
                .quiet_zone(true)
                .build();
            fs::write(path, svg_str).with_context(|| format!("saving {}", path.display()))?;
        }
        QrFormat::Png | QrFormat::Jpeg => {
            let fg = parse_hex(&settings.fg)?;
            let bg = parse_hex(&settings.bg)?;
            let img = code
                .render::<Rgb<u8>>()
                .min_dimensions(settings.size, settings.size)
                .dark_color(Rgb(fg))
                .light_color(Rgb(bg))
                .quiet_zone(true)
                .build();
            img.save(path)
                .with_context(|| format!("saving {}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_use_format_extension() {
        assert_eq!(
            file_name(Some(30), Some("/promo"), None, QrFormat::Png),
            "30-promo.png"
        );
        assert_eq!(
            file_name(Some(7), Some("/sale?x=1"), None, QrFormat::Svg),
            "7-sale-x-1.svg"
        );
        assert_eq!(
            file_name(Some(9), None, Some("My Link"), QrFormat::Jpeg),
            "9-My-Link.jpg"
        );
        assert_eq!(file_name(Some(5), Some("/"), None, QrFormat::Png), "link-5.png");
    }

    #[test]
    fn output_dir_is_per_workspace() {
        assert_eq!(output_dir(42), PathBuf::from("linkly-qr").join("42"));
    }

    #[test]
    fn colour_validation() {
        assert_eq!(normalize_color("#FF0000").as_deref(), Some("#ff0000"));
        assert_eq!(normalize_color("00ff00").as_deref(), Some("#00ff00"));
        assert_eq!(normalize_color("nope"), None);
        assert_eq!(normalize_color("#12345"), None);
    }

    #[test]
    fn writes_png_and_svg() {
        let dir = std::env::temp_dir().join(format!("linkly-qr-test-{}", std::process::id()));
        let mut s = QrSettings::default();

        let png = dir.join("t.png");
        write_qr("https://example.com/promo", &png, &s).unwrap();
        let bytes = std::fs::read(&png).unwrap();
        assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);

        s.format = QrFormat::Svg;
        let svg = dir.join("t.svg");
        write_qr("https://example.com/promo", &svg, &s).unwrap();
        let text = std::fs::read_to_string(&svg).unwrap();
        assert!(text.contains("<svg"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
