mod blit;
mod plugin;
mod retarget;
mod target;

#[cfg(test)]
mod test;

pub(in crate::net_qa) use plugin::EditorCapturePresentPlugin;
pub(in crate::net_qa) use target::{EditorQaCaptureTarget, aims_at};
