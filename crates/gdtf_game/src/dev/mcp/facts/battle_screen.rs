//! Whether the battle screen is up, as a command's availability check reads it.

crate::support_item! {
    /// Whether the battle screen is up, at any battlescape phase.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum BattleScreen {
        /// The game is showing the battle map.
        Open,
        /// The game is somewhere else.
        Closed,
    }
}
