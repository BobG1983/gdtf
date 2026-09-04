use bevy::prelude::*;

crate::support_item! {
    /// Root node of the combat log.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct CombatLogRoot;
}

crate::support_item! {
    /// One line of text in the combat log.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct CombatLogLine;
}
