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
    /// The stat block's portrait image.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatPortrait;
}

crate::support_item! {
    /// The stat block's ganger name text.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatName;
}

crate::support_item! {
    /// The stat block's faction text.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatFaction;
}

crate::support_item! {
    /// The stat block's stance text.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatStance;
}

crate::support_item! {
    /// The stat block's time-unit bar fill.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatTuBar;
}

crate::support_item! {
    /// The stat block's time-unit numeric label.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatTuLabel;
}

crate::support_item! {
    /// The stat block's hit-point bar fill.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatHpBar;
}

crate::support_item! {
    /// The stat block's hit-point numeric label.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatHpLabel;
}

crate::support_item! {
    /// The row of wound pips in the stat block.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatWoundsPips;
}

crate::support_item! {
    /// The container holding one line per wound.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatWoundList;
}

crate::support_item! {
    /// One wound line inside the wound list.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatWoundLine;
}

crate::support_item! {
    /// The container holding one line per lasting injury.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatInjuryList;
}

crate::support_item! {
    /// One injury line inside the injury list.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatInjuryLine;
}
