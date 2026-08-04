use bevy::prelude::Deref;

#[derive(Deref, Clone, Copy, Debug)]
pub(in crate::states) struct SceneLabel(&'static str);

impl SceneLabel {
    pub(in crate::states) const fn new(label: &'static str) -> Self {
        Self(label)
    }
}
