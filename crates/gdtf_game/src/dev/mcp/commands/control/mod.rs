//! View and battle controls a QA client drives the camera and the panels with.
pub(crate) mod level_down;
pub(crate) mod level_up;
pub(crate) mod look_at;
pub(crate) mod pan;
pub(crate) mod set_fire_mode;
pub(crate) mod support;
pub(crate) mod toggle_full_view;

#[cfg(test)]
mod test;

pub(crate) use level_down::ViewLevelDown;
pub(crate) use level_up::ViewLevelUp;
pub(crate) use look_at::ViewLookAt;
pub(crate) use pan::ViewPan;
pub(crate) use set_fire_mode::BattleSetFireMode;
pub(crate) use toggle_full_view::ViewToggleFullView;
