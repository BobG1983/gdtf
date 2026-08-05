//! Whether the file at a capture path is a complete, decodable PNG.

use crate::path::CapturePath;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShotFile {
    NotReady,
    Ready,
}

pub(super) fn inspect_shot(path: &CapturePath) -> ShotFile {
    let Ok(bytes) = std::fs::read(&**path) else {
        return ShotFile::NotReady;
    };
    if bytes.is_empty() {
        return ShotFile::NotReady;
    }
    match image::load_from_memory_with_format(&bytes, image::ImageFormat::Png) {
        Ok(_) => ShotFile::Ready,
        Err(_) => ShotFile::NotReady,
    }
}
