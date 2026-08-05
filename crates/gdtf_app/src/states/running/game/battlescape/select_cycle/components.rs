use bevy::prelude::*;

crate::support_item! {
    /// Root node of the selection-cycle control.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SelectCycleRoot;
}

crate::support_item! {
    /// Button that selects the next friendly ganger.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SelectNextButton;
}

crate::support_item! {
    /// Button that selects the previous friendly ganger.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SelectPrevButton;
}
