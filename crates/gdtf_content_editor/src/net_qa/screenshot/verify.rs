use gdtf_screenshot::CapturePath;

pub(in crate::net_qa) enum ShotFile {
        NotReady,
        Ready,
}

pub(in crate::net_qa) fn inspect_shot(path: &CapturePath) -> ShotFile {
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

#[cfg(test)]
mod test {
    use std::{fs, io::Cursor, path::Path};

    use gdtf_screenshot::CapturePath;

    use super::{ShotFile, inspect_shot};

        fn finished_png_bytes() -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([9, 9, 9, 255]));
        let mut bytes = Vec::new();
        let encoded = image.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png);
        assert!(encoded.is_ok(), "the fixture PNG must encode: {encoded:?}");
        bytes
    }

        fn write_file(path: &Path, bytes: &[u8]) {
        let written = fs::write(path, bytes);
        assert!(
            written.is_ok(),
            "the fixture file must be writable: {written:?}"
        );
    }

                    #[test]
    fn an_unfinished_file_is_never_ready() {
        let Ok(tmp) = tempfile::TempDir::new() else {
            unreachable!("a temp directory is available");
        };
        let absent = CapturePath::new(tmp.path().join("absent.png"));
        assert!(
            matches!(inspect_shot(&absent), ShotFile::NotReady),
            "a file that does not exist yet must not be ready",
        );

        let empty = CapturePath::new(tmp.path().join("empty.png"));
        write_file(&empty, &[]);
        assert!(
            matches!(inspect_shot(&empty), ShotFile::NotReady),
            "a zero-byte file must not be ready",
        );

        let full = finished_png_bytes();
        let truncated = CapturePath::new(tmp.path().join("truncated.png"));
        let head = full.len() / 3;
        write_file(&truncated, &full[..head]);
        assert!(
            matches!(inspect_shot(&truncated), ShotFile::NotReady),
            "a truncated PNG must not be ready — it does not decode",
        );

        let garbage = CapturePath::new(tmp.path().join("garbage.png"));
        write_file(&garbage, b"not a png at all, just some bytes");
        assert!(
            matches!(inspect_shot(&garbage), ShotFile::NotReady),
            "bytes that are not a PNG must not be ready",
        );
    }

        #[test]
    fn a_complete_png_is_ready() {
        let Ok(tmp) = tempfile::TempDir::new() else {
            unreachable!("a temp directory is available");
        };
        let path = CapturePath::new(tmp.path().join("finished.png"));
        write_file(&path, &finished_png_bytes());
        assert!(
            matches!(inspect_shot(&path), ShotFile::Ready),
            "a fully written PNG must be reported ready",
        );
    }
}
