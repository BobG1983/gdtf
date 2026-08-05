use bevy::prelude::*;

crate::support_item! {
    /// Main-menu button that starts a battle.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct BattlescapeButton;
}

crate::support_item! {
    /// Main-menu button that opens the options screen.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct OptionsButton;
}

crate::support_item! {
    /// Main-menu button that opens the hive scape.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct HiveScapeButton;
}

crate::support_item! {
    /// Main-menu button that quits the game.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct QuitButton;
}

crate::support_item! {
    /// The main-menu heading text.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MenuTitle;
}
