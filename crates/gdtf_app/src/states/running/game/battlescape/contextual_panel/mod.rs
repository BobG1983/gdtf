//! The battlescape CONTEXTUAL PANEL (GTW-294): the bottom-RIGHT cluster of the HUD that hosts
//! the situational acts on a downed neighbour.
//!
//! Per the `docs/ui_mockups/battlescape_mockup.png` bottom-right corner ("Contextual Buttons
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

/// Test-support re-exports for this panel (GTW-569 one-hop ledger): the contextual-panel's
/// root + button markers (GTW-294 / GTW-507 / GTW-525) the AC tests name through
/// `crate::test_support`. The crate-root ledger (`src/test_support.rs`) re-exports these
/// by explicit name directly from here — no intermediate `mod.rs` climb. `pub(crate)` on
/// the module (not `pub`) because the parent chain is `pub(crate)`, so a `pub mod` here
/// trips the workspace `unreachable_pub = deny`.
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::components::{
        ContextualPanelRoot, EnterEmplacementButton, ExecuteButton, ExitEmplacementButton,
        MeleeButton, OpenDoorButton, ShoveButton, StabilizeButton, ThrowGrenadeButton,
    };
}
