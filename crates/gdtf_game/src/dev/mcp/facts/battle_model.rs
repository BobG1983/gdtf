//! Whether the sim's battle resource is live, sampled into the facts value.

crate::support_item! {
    /// Whether the sim is holding a battle in progress.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum BattleModel {
        /// A battle is loaded in the sim.
        Present,
        /// No battle is loaded.
        Absent,
    }
}
