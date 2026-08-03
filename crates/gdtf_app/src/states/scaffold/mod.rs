mod label;
mod systems;

pub(in crate::states) use label::SceneLabel;
pub(in crate::states) use systems::{
    advance_state_to, insert_completion_marker, log_scene_enter, log_scene_exit,
    remove_scoped_resource,
};
