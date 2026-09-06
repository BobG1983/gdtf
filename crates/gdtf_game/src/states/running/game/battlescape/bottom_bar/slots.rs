use bevy::prelude::*;

/// The bar's children in row order, so a panel's place in the row does not depend
/// on which spawn system happens to run first.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::states::running::game::battlescape) enum BottomBarSlots {
    /// The weapon panel and the stance panel beside it, at the left end.
    WeaponPanel,
    /// The contextual act panel.
    ContextualPanel,
    /// The Next/Prev cluster, at the right end.
    SelectCycle,
}
