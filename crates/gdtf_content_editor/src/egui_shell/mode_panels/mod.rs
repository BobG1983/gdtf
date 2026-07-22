//! The per-mode RIGHT-form + CENTRAL-panel **dispatches** — split out of `shell.rs` at
//! the GTW-670 band boundary (module-layout: `shell.rs` sat one arm from the block band;
//! these two dispatches are the parts that grow an arm per Workbench mode, so they live
//! together here and the shell only changes when the PANEL LAYOUT does — the
//! continuing the `autoload.rs` / `textures.rs` split).
//!
//! Wiring-only module. The borrow context the shell builds once lives in [`ctx`]; the
//! RIGHT mode-form dispatch (panel-4 slot) lives in [`right`]; the CENTRAL
//! primary-panel dispatch (panel-5 slot, declared LAST per the egui order contract)
//! lives in [`central`].

mod central;
mod ctx;
mod right;

pub(super) use central::central_panel;
pub(super) use ctx::ModePanelsCtx;
pub(super) use right::right_panel;
