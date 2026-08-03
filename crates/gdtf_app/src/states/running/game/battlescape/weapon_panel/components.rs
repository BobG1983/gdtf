//! The weapon cluster is a battle-scoped `gdtf_ui` panel group laid out to the AUTHORITATIVE
use bevy::prelude::*;

crate::support_item! {
                                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponPanelRoot;
}

crate::support_item! {
                                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponContent;
}

crate::support_item! {
                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct CombinedWeaponPanel;
}

crate::support_item! {
                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponImage;
}

crate::support_item! {
                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponItemPanel;
}

crate::support_item! {
                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponItemButton;
}

crate::support_item! {
                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct AimPanel;
}

crate::support_item! {
                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct AimLabel;
}

crate::support_item! {
                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponNameText;
}

crate::support_item! {
                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct WeaponMagazineText;
}

crate::support_item! {
                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ReloadButton;
}
