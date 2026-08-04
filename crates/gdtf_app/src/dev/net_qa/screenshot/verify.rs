use gdtf_screenshot::CapturePath;

pub(in crate::dev::net_qa) enum ShotFile {
    NotReady,
    Ready,
}

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
