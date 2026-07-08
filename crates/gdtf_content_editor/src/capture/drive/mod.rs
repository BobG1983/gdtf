//! The capture drive's per-frame SYSTEMS — the env-forced overrides
//! (`force_capture_*`, in [`force`]) and the model staging drives (`drive_capture_*`, in
//! [`stage`]) that stage a legible, deterministic scene before the delegated
//! `gdtf_screenshot` settle + shot. Wiring-only module.

mod force;
mod stage;

pub(in crate::capture) use force::{
    force_capture_attachment, force_capture_mode, force_capture_terrain_kind, force_capture_view,
    force_capture_zoom,
};
pub(in crate::capture) use stage::{
    drive_capture_grid_size, drive_capture_paint_and_hover, drive_capture_selection,
};
