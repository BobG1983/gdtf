use bevy::prelude::*;

crate::support_item! {
                                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct CombatLogRoot;
}

crate::support_item! {
                                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct CombatLogLine;
}
