//! Whether the screen has caught up with the act log, as an availability check reads it.

crate::support_item! {
    /// Whether the screen has caught up with the act log.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum PlaybackCatchUp {
        /// The screen shows the whole log, so player input may proceed.
        CaughtUp,
        /// The screen is still playing the log back.
        Behind,
    }
}
