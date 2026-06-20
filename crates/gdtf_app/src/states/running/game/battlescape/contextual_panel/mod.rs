//! The battlescape CONTEXTUAL PANEL (GTW-294): the bottom-RIGHT cluster of the HUD that hosts
//! the situational acts on a downed neighbour.
//!
//! Per the `assets/ui_mockups/battlescape_mockup.png` bottom-right corner ("Contextual Buttons
//! go Here", to the right of the Stance column), this panel holds three themed buttons —
//! **Execute** / **Stabilize** / **Open Door**. The panel + all three buttons spawn
//! `Visibility::Hidden`; the live `detect_contextual_targets` system fills the
//! [`ContextualTargets`](components::ContextualTargets) seam each update and toggles each
//! button's `Visibility` IN PLACE when a valid downed neighbour is in reach, and
//! `contextual_button_intents` routes a press onto the shared act-intent seam. Execute +
//! Stabilize are live; Open Door stays hidden — it has no sim verb yet (no door / interactable
//! model), and delivering that model + act is out of scope for GTW-294 and tracked by the
//! follow-up GTW-315. View-only — it reads the input selection + writes the shared intent seam,
//! never a sim component directly.

mod components;
mod plugin;
mod systems;

pub(in crate::states::running::game::battlescape) use plugin::ContextualPanelPlugin;

// Test-support-only re-export of the contextual-panel's root + the three button markers (GTW-294),
// gated so the binary build is `unused`/`unreachable_pub`-clean (the action-bar / bottom-bar
// marker re-export chain precedent). Carries the markers up toward `crate::test_support`.
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{
        ContextualPanelRoot, ExecuteButton, OpenDoorButton, StabilizeButton,
    };
}
