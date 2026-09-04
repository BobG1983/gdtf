use bevy::prelude::*;

crate::support_item! {
    /// Root node of the inspect panel.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectPanelRoot;
}

crate::support_item! {
    /// The node the inspected ganger's stat block is mounted under.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectStatBlockHost;
}

crate::support_item! {
    /// The block shown when the inspected target is an object.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectBlock;
}

crate::support_item! {
    /// The inspected object's name text.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectText;
}

crate::support_item! {
    /// The inspected object's integrity bar fill.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectBar;
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::game::battlescape::inspect_panel) struct InspectObjectIntegrity;

crate::support_item! {
    /// The inspected object's hardness text.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectHardness;
}

crate::support_item! {
    /// The inspected object's cover-protection text.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectProtection;
}

crate::support_item! {
    /// The inspected object's height text.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectHeight;
}
