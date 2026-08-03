use bevy::prelude::*;

crate::support_item! {
                                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SelectCycleRoot;
}

crate::support_item! {
                                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SelectNextButton;
}

crate::support_item! {
                                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SelectPrevButton;
}
