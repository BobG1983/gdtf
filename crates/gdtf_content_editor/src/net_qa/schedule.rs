use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorNetQaSystems {
            Present,
        Gather,
}
