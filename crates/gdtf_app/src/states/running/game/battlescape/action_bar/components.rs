use bevy::prelude::*;

crate::support_item! {
                                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StanceStandingButton;
}

crate::support_item! {
                                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StanceKneelingButton;
}

crate::support_item! {
                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StanceProneButton;
}

crate::support_item! {
                                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct AimToggleButton;
}

crate::support_item! {
                                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModeSingleButton;
}

crate::support_item! {
                                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModeBurstButton;
}

crate::support_item! {
                                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModeFullButton;
}

crate::support_item! {
                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct LevelUpButton;
}

crate::support_item! {
                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct LevelDownButton;
}


crate::support_item! {
                                                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct EndTurnButton;
}

crate::support_item! {
                                                                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct FleeButton;
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::game::battlescape) struct ActionBarRoot;

crate::support_item! {
                                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StancePanelRoot;
}

crate::support_item! {
                                                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModePanelRoot;
}

crate::support_item! {
                                                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StanceControl;
}

crate::support_item! {
                                                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModeControl;
}
