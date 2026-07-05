//! GTW-278 / GTW-274 — the battlescape status panel + inspect panel (twins), driven
//! through the REAL app stack.
//!
//! These headless `GdtfTestAppBuilder` integration tests drive the genuine state machine
//! down to `BattleScapeState::BattleRunning`, where the real status-panel + inspect-panel
//! plugins spawn their shared stat blocks and their update systems repaint them under the
//! `BattleInProgress` gate. They cover:
//!
//! - **AC1 (status)** — the status panel renders the selected ganger's NAME +
//!   TU/HP `ProgressBar`s + Wounds `Pips` + wound-name list; the removed `LifeState` / `Weapon`
//!   lines are GONE; selecting a different ganger MUTATES in place.
//! - **Portrait** — the portrait node carries a `TextureAtlas` at the DETERMINISTIC index
//!   for the ganger's name (computed in-test from the same rule); a different name mutates
//!   the index on the SAME node.
//! - **AC2 (hover)** — `InspectTarget` over a ganger → its stat block; over a cover/object →
//!   the object block; over bare floor → the panel is hidden.

mod harness;
mod hover_gangers;
mod hover_harness;
mod hover_objects;
mod portrait;
mod stat_block;
