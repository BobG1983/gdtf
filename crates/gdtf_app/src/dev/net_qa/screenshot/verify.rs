//! Independent on-disk verification of a written screenshot (GTW-740).
//!
//! The GPU readback + PNG encode is asynchronous, so a file can appear on disk before it is
//! completely written. A bare `exists()` check would report a truncated, half-flushed file
//! as saved — so [`inspect_shot`] additionally requires the bytes to be NON-EMPTY and to
//! DECODE as a PNG before the pump reports the capture landed.

use gdtf_screenshot::CapturePath;

/// The outcome of inspecting a screenshot's on-disk state.
pub(in crate::dev::net_qa) enum ShotFile {
    /// The file is absent, empty, or not yet fully decodable — keep polling.
    NotReady,
    /// The file exists, is non-empty, and decodes as a PNG — safe to report saved.
    Ready,
}

/// Inspect the screenshot at `path`: [`Ready`](ShotFile::Ready) only when it exists, has
/// non-zero size, AND decodes as a PNG; [`NotReady`](ShotFile::NotReady) otherwise (still
/// being written, or a partial flush).
///
/// Reading + decoding the bytes is the INDEPENDENT proof the capture landed WHOLE — the
/// pump never replies "saved" on a file it has not decoded.
pub(in crate::dev::net_qa) fn inspect_shot(path: &CapturePath) -> ShotFile {
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
