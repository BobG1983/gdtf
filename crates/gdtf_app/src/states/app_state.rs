use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) enum AppState {
    #[default]
    Init,
    Load,
    Intro,
    Running,
    Teardown,
}
