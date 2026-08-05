//! Whether a battle is running, as a command's availability check reads it.

crate::support_item! {
    /// Whether a battle is running right now.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum BattleActivity {
        /// A battle is in its running phase.
        Running,
        /// No battle is in its running phase.
        NotRunning,
    }
}
