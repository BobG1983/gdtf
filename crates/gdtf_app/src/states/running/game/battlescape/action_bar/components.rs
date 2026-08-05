use bevy::prelude::*;

crate::support_item! {
    /// Button that sets the selected ganger's stance to standing.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StanceStandingButton;
}

crate::support_item! {
    /// Button that sets the selected ganger's stance to kneeling.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StanceKneelingButton;
}

crate::support_item! {
    /// Button that sets the selected ganger's stance to prone.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StanceProneButton;
}

crate::support_item! {
    /// Button that toggles aimed fire.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct AimToggleButton;
}

crate::support_item! {
    /// Button that selects single-shot fire mode.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModeSingleButton;
}

crate::support_item! {
    /// Button that selects burst fire mode.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModeBurstButton;
}

crate::support_item! {
    /// Button that selects full-auto fire mode.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModeFullButton;
}

crate::support_item! {
    /// Button that raises the camera one level.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct LevelUpButton;
}

crate::support_item! {
    /// Button that lowers the camera one level.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct LevelDownButton;
}

crate::support_item! {
    /// Button that ends the player's turn.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct EndTurnButton;
}

crate::support_item! {
    /// Button that flees the battle.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct FleeButton;
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::game::battlescape) struct ActionBarRoot;

crate::support_item! {
    /// Root node of the stance panel.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StancePanelRoot;
}

crate::support_item! {
    /// Root node of the fire-mode panel.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModePanelRoot;
}

crate::support_item! {
    /// One stance segment inside the stance panel.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StanceControl;
}

crate::support_item! {
    /// One fire-mode segment inside the mode panel.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModeControl;
}
