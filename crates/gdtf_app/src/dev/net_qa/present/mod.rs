mod retarget;

#[cfg(test)]
mod test;

pub(in crate::dev::net_qa) use retarget::{mark_ui_default_camera, retarget_cameras_to_offscreen};
