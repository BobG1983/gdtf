//! GTW-275 — the battlescape weapon panel (bottom-left) + the LIVE reload wiring, driven
//! through the REAL app stack.
//!
//! These headless `GdtfTestAppBuilder` integration tests drive the genuine state machine
//! down to `BattleScapeState::BattleRunning`, where the real weapon-panel plugin spawns its
//! tree and its update system repaints it under the `BattleInProgress` gate. They cover:
//!
//! - **AC5** — the weapon name + magazine `"cur/max"` text reflect the selected ganger's
//!   components; selecting a different ganger MUTATES in place (stable widget ids).
//! - **AC6** — the LIVE Reload button is shown when the weapon has a magazine (`size > 0`),
//!   hidden with no weapon; pressing it emits an `ActIntent::Reload` → `ReloadRequested`
//!   through the `gdtf_battle_input` seam.
//! - **AC9** — no selection / no weapon → the weapon content is `Visibility::Hidden`.
//! - **AC8** — the action-bar no longer carries a Reload button (the removed marker is gone).
//! - **GTW-298 rework** — the cluster matches the AUTHORITATIVE structure: the Overall Weapon
//!   Panel 2×2 grid (Combined / Firemode / Item / Aim) + the separate Stance Panel, the
//!   relocated firemode / aim / stance controls present in their new panels, and all sizing
//!   RESPONSIVE (`Val::Vw`/`Val::Vh`/`Val::Percent`, NOT a fixed `Val::Px`); the empty-state
//!   hide uses `Display::None` (removed from layout) so a hidden weapon block takes no space.

mod aim_stance_layout;
mod bottom_bar;
mod harness;
mod panel_content;
mod reload_button;
mod structure;
