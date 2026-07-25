//! The harness's state: which two UI stacks are under comparison, and which of them is
//! rendering right now.

use bevy::prelude::*;

crate::support_item! {
    /// One UI stack the game can render a screen through.
    ///
    /// A named domain enum, never a bool or a string (the no-bare-types rule): "which stack
    /// is live" is a domain value the keyboard, the wire, and the snapshot all speak.
    #[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
    enum UiStackId {
        /// Bevy's retained-mode `bevy_ui` — the stack the shipping game HUD is built in,
        /// and the harness's default.
        #[default]
        BevyUi,
        /// The immediate-mode egui stack.
        Egui,
    }
}

crate::support_item! {
    /// The two stacks currently under comparison, in the harness's own order.
    ///
    /// Held as a PAIR rather than assumed, because "which two are being compared" is a
    /// property of the comparison surface, not a constant: the toggle flips between these
    /// two, so a future comparison that pits a different pair against each other changes
    /// this value and nothing else.
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
    struct UiStackPair {
        /// The stack the comparison starts from.
        baseline:  UiStackId,
        /// The stack being judged against the baseline.
        candidate: UiStackId,
    }
}

impl UiStackPair {
    crate::support_item! {
        /// Build a comparison pair.
        #[must_use]
        const fn new(baseline: UiStackId, candidate: UiStackId) -> Self {
            Self {
                baseline,
                candidate,
            }
        }
    }

    crate::support_item! {
        /// The stack the comparison starts from.
        #[must_use]
        const fn baseline(self) -> UiStackId {
            self.baseline
        }
    }

    crate::support_item! {
        /// The stack being judged against the baseline.
        #[must_use]
        const fn candidate(self) -> UiStackId {
            self.candidate
        }
    }

    /// The OTHER member of the pair — what a toggle from `stack` lands on.
    ///
    /// A stack outside the pair (impossible while the pair is the only writer of
    /// [`UiStack::live`]) toggles to the baseline, so the harness can never wedge on a
    /// stack it does not render.
    fn other_than(self, stack: UiStackId) -> UiStackId {
        if stack == self.baseline {
            self.candidate
        } else {
            self.baseline
        }
    }
}

impl Default for UiStackPair {
    /// The GTW-796 comparison: the shipping `bevy_ui` HUD against the egui candidate.
    fn default() -> Self {
        Self::new(UiStackId::BevyUi, UiStackId::Egui)
    }
}

crate::support_item! {
    /// The DEV UI-stack swap harness's state — the pair under comparison plus the live one.
    ///
    /// The ONE source of truth for "which stack is rendering": the keyboard shortcut, the
    /// `net_qa` `SwapUiStack` intent, and both stacks' own on-screen swap buttons all reach
    /// this resource through the same latch, and every spawn / draw decision reads it.
    #[derive(Resource, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
    struct UiStack {
        /// The two stacks under comparison.
        comparison: UiStackPair,
        /// Which of them is rendering right now.
        live:       UiStackId,
    }
}

impl UiStack {
    crate::support_item! {
        /// Which stack is rendering right now.
        #[must_use]
        const fn live(self) -> UiStackId {
            self.live
        }
    }

    crate::support_item! {
        /// The two stacks under comparison.
        #[must_use]
        const fn comparison(self) -> UiStackPair {
            self.comparison
        }
    }

    crate::support_item! {
        /// Whether `stack` is the live one — the read every spawn / draw gate makes.
        #[must_use]
        fn is_live(self, stack: UiStackId) -> bool {
            self.live == stack
        }
    }

    /// Make `stack` live. Absolute, so setting the already-live stack is a no-op.
    pub(super) const fn set_live(&mut self, stack: UiStackId) {
        self.live = stack;
    }

    /// Flip to the other member of the compared pair.
    pub(super) fn toggle(&mut self) {
        self.live = self.comparison.other_than(self.live);
    }
}

#[cfg(test)]
mod test {
    use super::{UiStack, UiStackId, UiStackPair};

    /// A toggle flips between exactly the two compared stacks, and a second toggle returns —
    /// the property the on-screen swap buttons and the keyboard shortcut both rest on.
    #[test]
    fn toggling_flips_between_the_compared_pair() {
        let mut stack = UiStack::default();
        assert_eq!(stack.live(), UiStackId::BevyUi);
        stack.toggle();
        assert_eq!(stack.live(), UiStackId::Egui);
        stack.toggle();
        assert_eq!(stack.live(), UiStackId::BevyUi);
    }

    /// Setting a stack live is ABSOLUTE, not a flip: naming the already-live stack twice
    /// leaves it live (what the wire `SwapUiStack` contract promises), where two toggles
    /// would have returned to the start.
    #[test]
    fn setting_live_is_absolute_not_a_flip() {
        let mut stack = UiStack::default();
        stack.set_live(UiStackId::Egui);
        stack.set_live(UiStackId::Egui);
        assert_eq!(stack.live(), UiStackId::Egui);
        assert!(stack.is_live(UiStackId::Egui));
        assert!(!stack.is_live(UiStackId::BevyUi));
    }

    /// The pair is reported as authored — baseline and candidate keep their sides, so a
    /// snapshot reader can tell which stack is the incumbent.
    #[test]
    fn the_pair_keeps_its_sides() {
        let pair = UiStackPair::default();
        assert_eq!(pair.baseline(), UiStackId::BevyUi);
        assert_eq!(pair.candidate(), UiStackId::Egui);
        assert_eq!(pair.other_than(UiStackId::BevyUi), UiStackId::Egui);
        assert_eq!(pair.other_than(UiStackId::Egui), UiStackId::BevyUi);
    }
}
