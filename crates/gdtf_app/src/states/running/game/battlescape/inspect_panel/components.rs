use bevy::prelude::*;

crate::support_item! {
                                                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectPanelRoot;
}

crate::support_item! {
                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectStatBlockHost;
}

crate::support_item! {
                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectBlock;
}

crate::support_item! {
                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectText;
}

crate::support_item! {
                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectBar;
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::game::battlescape::inspect_panel) struct InspectObjectIntegrity;

crate::support_item! {
                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectHardness;
}

crate::support_item! {
                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectProtection;
}

crate::support_item! {
                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectHeight;
}
