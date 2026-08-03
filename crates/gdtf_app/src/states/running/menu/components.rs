use bevy::prelude::*;

crate::support_item! {
                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct BattlescapeButton;
}

crate::support_item! {
                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct OptionsButton;
}

crate::support_item! {
                                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct HiveScapeButton;
}

crate::support_item! {
                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct QuitButton;
}

crate::support_item! {
                                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MenuTitle;
}
