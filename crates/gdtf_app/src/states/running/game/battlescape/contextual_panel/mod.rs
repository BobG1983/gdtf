//! The battlescape CONTEXTUAL PANEL (GTW-294; generic per-act machinery since
//! GTW-571): the bottom-RIGHT cluster of the HUD that hosts the situational acts.
//!
//! Per the `docs/ui_mockups/battlescape_mockup.png` bottom-right corner ("Contextual
//! Buttons go Here", to the right of the Stance column), this panel holds one themed
//! button per registered contextual act — **Execute** / **Stabilize** / **Melee** /
//! **Shove** / **Open Door** / **Enter** / **Exit Emplacement** / **Throw**. The panel
//! and every button spawn `Visibility::Hidden`; each act's OFFER scan fills its
//! [`ContextualOffer`](seam::ContextualOffer) each update and the generic toggle flips
//! its button's `Visibility` IN PLACE when a valid target is offered, and the generic
//! press router pushes the target onto the act's buffered
//! [`PendingContextualIntents`](gdtf_battle_input::contextual::PendingContextualIntents)
//! queue (drained same-frame by the input layer's generic drain — the GTW-571 Q5
//! invariant). View-only — it reads the input selection + writes the per-act intent
//! queues, never a sim component directly. Adding a contextual act adds ONE module
//! under [`acts`] plus ONE registration line in [`plugin`] (see
//! `docs/authoring/contextual-act-recipe.md`).

mod acts;
mod components;
mod plugin;
mod registrar;
mod seam;
mod systems;

pub(in crate::states::running::game::battlescape) use plugin::ContextualPanelPlugin;

/// Test-support re-exports for this panel (GTW-569 one-hop ledger): the contextual
/// panel's root marker plus each act's button marker (declared in the act's own module
/// under [`acts`] — GTW-571) that the AC tests name through `crate::test_support`. The
/// crate-root ledger (`src/test_support.rs`) re-exports these by explicit name directly
/// from here — no intermediate `mod.rs` climb. `pub(crate)` on the module (not `pub`)
/// because the parent chain is `pub(crate)`, so a `pub mod` here trips the workspace
/// `unreachable_pub = deny`.
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::{
        acts::{
            enter_emplacement::EnterEmplacementButton, execute::ExecuteButton,
            exit_emplacement::ExitEmplacementButton, melee::MeleeButton, open_door::OpenDoorButton,
            shove::ShoveButton, stabilize::StabilizeButton, throw_grenade::ThrowGrenadeButton,
        },
        components::ContextualPanelRoot,
    };
}
