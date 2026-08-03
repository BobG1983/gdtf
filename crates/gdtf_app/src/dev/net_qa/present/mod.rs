mod blit;
mod plugin;
mod retarget;
mod target;

#[cfg(test)]
mod test;

pub(in crate::dev::net_qa) use plugin::CapturePresentPlugin;
pub(in crate::dev::net_qa) use target::QaCaptureTarget;
