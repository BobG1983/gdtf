use bevy::prelude::*;

#[derive(Component, Clone, Copy, Debug)]
pub(in crate::states::running::game::battlescape) struct StatBlockRefs {
    pub portrait:    Entity,
    pub name:        Entity,
    pub faction:     Entity,
    pub stance:      Entity,
    pub tu_bar:      Entity,
    pub tu_label:    Entity,
    pub hp_bar:      Entity,
    pub hp_label:    Entity,
    pub wounds:      Entity,
    pub wound_list:  Entity,
    pub injury_list: Entity,
}

crate::support_item! {
                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatPortrait;
}

crate::support_item! {
                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatName;
}

crate::support_item! {
                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatFaction;
}

crate::support_item! {
                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatStance;
}

crate::support_item! {
                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatTuBar;
}

crate::support_item! {
                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatTuLabel;
}

crate::support_item! {
                #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatHpBar;
}

crate::support_item! {
                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatHpLabel;
}

crate::support_item! {
                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatWoundsPips;
}

crate::support_item! {
                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatWoundList;
}

crate::support_item! {
                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatWoundLine;
}

crate::support_item! {
                                    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatInjuryList;
}

crate::support_item! {
                            #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatInjuryLine;
}
