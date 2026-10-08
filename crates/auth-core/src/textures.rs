//! Skins and capes: validation, storage (content-addressed PNGs under
//! `data/textures/`) and rendering of player heads for avatars.

#[derive(Debug)]
pub enum TextureError {
    BadRequest(String),
    Io(std::io::Error),
}
impl TextureError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }
}
impl std::fmt::Display for TextureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(message) => f.write_str(message),
            Self::Io(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for TextureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::BadRequest(_) => None,
        }
    }
}
impl From<std::io::Error> for TextureError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
pub type TextureResult<T> = Result<T, TextureError>;

use image::{imageops, ImageFormat, RgbaImage};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Skin,
    Cape,
}

fn decode(bytes: &[u8]) -> TextureResult<RgbaImage> {
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(TextureError::bad_request("image too large (2 MB max)"));
    }
    let img = image::load_from_memory_with_format(bytes, ImageFormat::Png)
        .map_err(|_| TextureError::bad_request("that isn't a valid PNG image"))?;
    Ok(img.to_rgba8())
}

fn check_size(kind: Kind, w: u32, h: u32) -> TextureResult<()> {
    let ok = match kind {
        // The vanilla client only accepts these two layouts.
        Kind::Skin => (w, h) == (64, 64) || (w, h) == (64, 32),
        // 64x32 is standard; HD capes are multiples of it; 22x17 is the old format.
        Kind::Cape => (w.is_multiple_of(64) && h * 2 == w && w <= 1024) || (w, h) == (22, 17),
    };
    if ok {
        Ok(())
    } else {
        Err(TextureError::bad_request(match kind {
            Kind::Skin => format!("skins must be 64×64 (or legacy 64×32) pixels — this one is {w}×{h}"),
            Kind::Cape => format!("capes must be 64×32 pixels — this one is {w}×{h}"),
        }))
    }
}

/// Validate, re-encode (strips metadata and anything smuggled inside the
/// file) and store a texture. Returns its hash.
pub fn store(dir: &Path, kind: Kind, bytes: &[u8]) -> TextureResult<String> {
    let img = decode(bytes)?;
    check_size(kind, img.width(), img.height())?;
    let mut out = Vec::new();
    img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
        .map_err(|e| TextureError::bad_request(format!("couldn't process image: {e}")))?;
    let hash = hex::encode(Sha256::digest(&out));
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("{hash}.png"));
    if !path.exists() {
        std::fs::write(&path, &out)?;
    }
    Ok(hash)
}

pub fn path(dir: &Path, hash: &str) -> Option<PathBuf> {
    // Hashes are hex only — never let a request escape the directory.
    (hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())).then(|| dir.join(format!("{hash}.png")))
}

/// Front view of the head (face + hat layer), scaled with nearest-neighbour
/// so pixels stay crisp.
pub fn render_head(skin_png: &[u8], size: u32) -> TextureResult<Vec<u8>> {
    let skin = decode(skin_png)?;
    let mut face = imageops::crop_imm(&skin, 8, 8, 8, 8).to_image();
    if skin.height() >= 16 && skin.width() >= 48 {
        let hat = imageops::crop_imm(&skin, 40, 8, 8, 8).to_image();
        // Some skins fill the hat layer with an opaque colour; ignore it then.
        let opaque_hat = hat.pixels().all(|p| p[3] == 255);
        if !opaque_hat {
            imageops::overlay(&mut face, &hat, 0, 0);
        }
    }
    let size = size.clamp(8, 512);
    let scaled = imageops::resize(&face, size, size, imageops::FilterType::Nearest);
    let mut out = Vec::new();
    scaled.write_to(&mut Cursor::new(&mut out), ImageFormat::Png).map_err(|e| TextureError::bad_request(e.to_string()))?;
    Ok(out)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn png(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        let img = RgbaImage::from_pixel(w, h, image::Rgba(rgba));
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png).unwrap();
        out
    }

    #[test]
    fn validates_and_stores() {
        let dir = tempfile::tempdir().unwrap();
        let h1 = store(dir.path(), Kind::Skin, &png(64, 64, [200, 10, 10, 255])).unwrap();
        let h2 = store(dir.path(), Kind::Skin, &png(64, 64, [200, 10, 10, 255])).unwrap();
        assert_eq!(h1, h2, "content-addressed");
        assert!(path(dir.path(), &h1).unwrap().exists());
        assert!(store(dir.path(), Kind::Skin, &png(32, 32, [0, 0, 0, 255])).is_err());
        assert!(store(dir.path(), Kind::Cape, &png(64, 32, [0, 0, 0, 255])).is_ok());
        assert!(store(dir.path(), Kind::Skin, b"not a png").is_err());
        assert!(path(dir.path(), "../../etc/passwd").is_none());
    }

    #[test]
    fn renders_heads() {
        let mut skin = RgbaImage::from_pixel(64, 64, image::Rgba([0, 0, 0, 0]));
        for x in 8..16 {
            for y in 8..16 {
                skin.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        skin.put_pixel(40, 8, image::Rgba([0, 0, 255, 255])); // one hat pixel
        let mut bytes = Vec::new();
        skin.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).unwrap();
        let head = image::load_from_memory(&render_head(&bytes, 64).unwrap()).unwrap().to_rgba8();
        assert_eq!(head.dimensions(), (64, 64));
        assert_eq!(head.get_pixel(0, 0).0, [0, 0, 255, 255], "hat overlays the face");
        assert_eq!(head.get_pixel(63, 63).0, [255, 0, 0, 255]);
    }

    #[test]
    fn legacy_layouts_limits_and_filesystem_failure_keep_error_classes() {
        assert!(check_size(Kind::Skin, 64, 32).is_ok());
        for (w, h) in [(64, 32), (128, 64), (1024, 512), (22, 17)] {
            assert!(check_size(Kind::Cape, w, h).is_ok());
        }
        assert!(matches!(check_size(Kind::Cape, 1088, 544), Err(TextureError::BadRequest(_))));
        let error = decode(&vec![0; 2 * 1024 * 1024 + 1]).unwrap_err();
        assert!(matches!(error, TextureError::BadRequest(ref message) if message == "image too large (2 MB max)"));
        let directory = tempfile::tempdir().unwrap();
        let blocked = directory.path().join("synthetic-file");
        std::fs::write(&blocked, b"not a directory").unwrap();
        assert!(matches!(store(&blocked, Kind::Skin, &png(64, 32, [1, 2, 3, 255])), Err(TextureError::Io(_))));
    }

    #[test]
    fn opaque_hat_is_ignored_and_avatar_sizes_remain_clamped() {
        let skin = png(64, 32, [1, 2, 3, 255]);
        for (requested, expected) in [(0, 8), (4096, 512)] {
            let bytes = render_head(&skin, requested).unwrap();
            let head = image::load_from_memory(&bytes).unwrap().to_rgba8();
            assert_eq!(head.dimensions(), (expected, expected));
            assert_eq!(head.get_pixel(0, 0).0, [1, 2, 3, 255]);
        }
    }
}
