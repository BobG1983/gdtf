//! The DEV-ONLY UI-stack coexistence proof-of-concept (GTW-819): one `bevy_ui` button and
//! one egui button, rendered and clickable at the same time, in one [`AppState`](crate::states::AppState).
//!
//! It exists to answer ONE question before the GTW-796 comparison epic builds on it — can the
//! two UI stacks genuinely be alive together (render passes, pointer capture, context
//! lifetime, feature gating)? — and deliberately contains nothing else: no toggle, no swap
//! harness, no theming, no second widget.
//!
//! See [`plugin`] for which state it attaches to and why, and for the `EguiPlugin`-ownership
//! constraint two egui-using dev affordances impose on each other.

mod bevy_ui_button;
mod counters;
mod egui_button;
mod egui_context;
mod plugin;

// `UiCoexistencePlugin` is the only item the binary consumes (via the dev aggregate plugin,
// `crate::dev::plugin`), so it is re-exported in BOTH configurations, at the `test-support`
// visibility flip the type itself uses — the stepper's own plugin re-export precedent.
crate::support_use!(plugin::UiCoexistencePlugin;);

// The rest of the surface exists ONLY for the GTW-819 integration suite: the per-stack tallies
// it asserts on, the `bevy_ui` button's marker (to find the one entity whose `Interaction` it
// drives) and the egui button's pinned position (to aim the synthetic egui pointer without
// guessing at egui's frame margins). Each is reached in-crate through `super::…` by its own
// sibling systems, so these re-exports are gated on `test-support` — unconditional ones would
// be unused `pub(crate) use` warnings in the binary build.
#[cfg(feature = "test-support")]
pub use bevy_ui_button::CoexistenceBevyUiButton;
#[cfg(feature = "test-support")]
pub use counters::{ClickCount, UiStackClicks};
#[cfg(feature = "test-support")]
pub use egui_button::EGUI_PANEL_POS;
