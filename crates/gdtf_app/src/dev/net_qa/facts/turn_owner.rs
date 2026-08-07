//! Whose turn it is, as a command's availability check reads it.

crate::support_item! {
    /// Whether the acting faction is the one the player commands.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum TurnOwner {
        /// The player's own faction is acting.
        Player,
        /// Another faction is acting, or no battle has a faction acting at all.
        OtherFaction,
    }
}
