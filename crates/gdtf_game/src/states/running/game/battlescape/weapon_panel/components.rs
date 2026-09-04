//! The weapon cluster is a battle-scoped `gdtf_ui` panel group laid out to the AUTHORITATIVE
use bevy::prelude::*;

crate::support_item! {
    /// Root node of the weapon cluster.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponPanelRoot;
}

crate::support_item! {
    /// The weapon cluster's scrolling content area.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponContent;
}

crate::support_item! {
    /// The panel showing the selected ganger's combined weapon state.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct CombinedWeaponPanel;
}

crate::support_item! {
    /// The weapon's icon image.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponImage;
}

crate::support_item! {
    /// One weapon entry's panel.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponItemPanel;
}

crate::support_item! {
    /// The clickable button on one weapon entry.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponItemButton;
}

crate::support_item! {
    /// The panel holding the aim readout.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct AimPanel;
}

crate::support_item! {
    /// The aim readout text.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct AimLabel;
}

crate::support_item! {
    /// The weapon name text.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponNameText;
}

crate::support_item! {
    /// The loaded/capacity magazine text.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponMagazineText;
}

crate::support_item! {
    /// The reload button.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ReloadButton;
}
