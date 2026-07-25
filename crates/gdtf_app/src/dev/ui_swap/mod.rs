//! The DEV-ONLY UI-stack swap harness (GTW-816): one comparison panel, rendered through
//! either `bevy_ui` or egui, with a live swap driven identically by a keyboard shortcut, by
//! each stack's own on-screen button, and by the `net_qa`
//! [`SwapUiStack`](gdtf_qa_protocol::intent::NetIntent::SwapUiStack) intent.
//!
//! It exists so the GTW-796 comparison children (the main-menu, battle-HUD and editor
//! comparisons) share ONE swap mechanism instead of each inventing a toggle, and so an
//! agent can drive the same swap a developer drives and assert behavioural parity across
//! it. Built on the GTW-819 coexistence result: the two stacks genuinely do render together
//! in one app, in one `AppState`.
//!
//! Members, one concern per file:
//!
//! - [`stack`] — the harness state ([`UiStack`](stack::UiStack)): the compared pair plus the
//!   live stack.
//! - [`latch`] — the ONE swap funnel every trigger writes, and the single system that
//!   applies it (the multipass double-swap guard lives here).
//! - [`keyboard`] — the developer-facing shortcut.
//! - [`bevy_ui_panel`] / [`egui_panel`] — the two renderings of the same one control, and
//!   the spawn / despawn lifecycle that keeps only the live one on screen.
//! - [`egui_context`] — pins the primary egui context to the game's UI camera, without
//!   which the egui stack is invisible to `net_qa` screenshots.
//! - [`plugin`] — the wiring, the run-condition guards, and the single `EguiPlugin`
//!   ownership decision.

mod bevy_ui_panel;
mod egui_context;
mod egui_panel;
mod keyboard;
mod latch;
mod plugin;
mod stack;

// `UiSwapHarnessPlugin` is consumed by the dev aggregate plugin (`crate::dev::plugin`), so
// it is re-exported in BOTH configurations at the `test-support` visibility flip the type
// itself uses — the stepper's and the coexistence spike's own plugin re-export precedent.
crate::support_use!(plugin::UiSwapHarnessPlugin;);

// The swap latch is written by the `net_qa` `SwapUiStack` dispatch
// (`crate::dev::net_qa::ui_swap`) and the live stack is read by the T5 snapshot, both
// OUTSIDE this module — so these two ride `any(test-support, net_qa)`: `support_use!`
// widens them to `pub` under `test-support` (the ledger needs them) and `pub(crate)`
// otherwise (the net_qa dispatch names them crate-wide). Gated on `any(...)` rather than
// unconditional so a plain `dev_tools`-only build stays warning-clean — every in-module
// consumer reaches them through `super::…`.
#[cfg(any(feature = "test-support", feature = "net_qa"))]
crate::support_use!(latch::PendingUiStackSwap;);
#[cfg(any(feature = "test-support", feature = "net_qa"))]
crate::support_use!(stack::{UiStack, UiStackId};);
// The compared PAIR is reached out-of-module only by the integration suite: the net_qa
// snapshot read gets it back from `UiStack::comparison()` without ever naming the type, so
// an `any(...)`-gated re-export would be an unused import in a net_qa-only build.
// The rest of the surface exists ONLY for the GTW-816 integration suites: the `bevy_ui`
// rendering's markers (to find the panel entities and drive the one button's `Interaction`)
// and the egui rendering's pinned position (to aim the synthetic egui pointer without
// guessing at egui's frame margins).
#[cfg(feature = "test-support")]
pub use bevy_ui_panel::{UI_SWAP_CAPTION, UiSwapBevyUiButton, UiSwapBevyUiPanel};
#[cfg(feature = "test-support")]
pub use egui_panel::EGUI_SWAP_PANEL_POS;
// The shortcut's key, so the in-engine / headless keyboard test taps the SHIPPED binding
// rather than re-declaring a literal that could silently drift from it.
#[cfg(feature = "test-support")]
pub use keyboard::UI_STACK_SWAP_KEY;
#[cfg(feature = "test-support")]
pub use stack::UiStackPair;
