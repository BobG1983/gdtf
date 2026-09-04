//! Whether the procgen stepper owns generation, as an availability check reads it.

crate::support_item! {
    /// Whether the procgen stepper owns situation generation right now.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum StepperActivity {
        /// The stepper drives generation one stage at a time.
        Stepping,
        /// Generation runs itself, or the stepper is not in this build.
        NotStepping,
    }
}
