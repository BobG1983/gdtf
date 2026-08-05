//! Whether the battle presenter is drawing a live battle.

crate::support_item! {
    /// Whether a running battle is loaded, so the presenter's reads have an answer.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum PresenterReadiness {
        /// A battle is running and its sim state is live.
        Ready,
        /// Nothing is being presented.
        NotReady,
    }
}
