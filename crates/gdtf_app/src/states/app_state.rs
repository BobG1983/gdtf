use bevy::prelude::*;

crate::support_item! {
        #[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
    enum AppState {
                #[default]
        Init,
                Load,
                Intro,
                Running,
                Teardown,
    }
}
