//! The two observable effects the spike's buttons produce: one activation tally per UI
//! stack, held side by side in ONE resource so a test (and a screenshot) can show that a
//! click on one stack's button moved that stack's tally and left the other's alone.

use bevy::prelude::*;

crate::support_item! {
    /// How many times one UI stack's spike button has been activated.
    ///
    /// A newtype rather than a bare `u32` (the no-bare-types rule): a tally of activations
    /// is a domain value here — it is the ENTIRE observable the coexistence proof rests on.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deref)]
    struct ClickCount(u32);
}

impl ClickCount {
    crate::support_item! {
        /// The tally as a plain count, for formatting into a label.
        #[must_use]
        const fn get(self) -> u32 {
            self.0
        }
    }

    /// Records one activation (saturating, so a wedged test loop can never overflow).
    pub(super) const fn bump(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}

crate::support_item! {
    /// The spike's per-stack activation tallies — the resource both buttons write.
    ///
    /// ONE resource, two fields: an assertion that the `bevy_ui` click moved
    /// [`bevy_ui`](Self::bevy_ui) while [`egui`](Self::egui) stood still (and the mirror of
    /// it) is exactly the "neither stack swallows the other's clicks" proof, and it cannot
    /// be satisfied by a single shared counter.
    #[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
    struct UiStackClicks {
        /// Activations of the `bevy_ui` button.
        bevy_ui: ClickCount,
        /// Activations of the egui button.
        egui:    ClickCount,
    }
}

impl UiStackClicks {
    crate::support_item! {
        /// The `bevy_ui` button's tally.
        #[must_use]
        const fn bevy_ui(&self) -> ClickCount {
            self.bevy_ui
        }
    }

    crate::support_item! {
        /// The egui button's tally.
        #[must_use]
        const fn egui(&self) -> ClickCount {
            self.egui
        }
    }

    /// Records one `bevy_ui` activation.
    pub(super) const fn bump_bevy_ui(&mut self) {
        self.bevy_ui.bump();
    }

    /// Records one egui activation.
    pub(super) const fn bump_egui(&mut self) {
        self.egui.bump();
    }
}

#[cfg(test)]
mod test {
    use super::UiStackClicks;

    /// The two tallies are genuinely independent — bumping one never moves the other.
    /// (The app-level proof that each BUTTON reaches its own tally lives in the
    /// `ui_coexistence` integration suite; this pins the data shape it rests on.)
    #[test]
    fn each_stack_has_its_own_tally() {
        let mut clicks = UiStackClicks::default();
        clicks.bump_bevy_ui();
        assert_eq!(clicks.bevy_ui().get(), 1);
        assert_eq!(clicks.egui().get(), 0);
        clicks.bump_egui();
        clicks.bump_egui();
        assert_eq!(clicks.bevy_ui().get(), 1);
        assert_eq!(clicks.egui().get(), 2);
    }
}
