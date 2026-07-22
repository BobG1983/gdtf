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
//!   through the `gdtf_battle_input` act-intent queue.
//! - **AC9** — no selection / no weapon → the weapon content is `Visibility::Hidden`.
//! - **AC8** — the action-bar no longer carries a Reload button (the removed marker is gone).
//! - **GTW-298 rework** — the cluster matches the AUTHORITATIVE structure: the Overall Weapon
//!   Panel 2×2 grid (Combined / Firemode / Item / Aim) + the separate Stance Panel, the
//!   relocated firemode / aim / stance controls present in their new panels, and all sizing
//!   RESPONSIVE (`Val::Vw`/`Val::Vh`/`Val::Percent`, NOT a fixed `Val::Px`); the empty-state
//!   hide uses `Display::None` (removed from layout) so a hidden weapon block takes no space.
//! - **GTW-733** — the weapon block is wide enough for a real shipped weapon name: Reload moved
//!   out of the name's row into its own row below, so the two never share pixels. Unlike the rest
//!   of this file's `MinimalPlugins`/declared-`Node`-field tests, `layout_geometry` drives a
//!   SEPARATE REAL-layout harness (`real_layout_harness`, a headless `DefaultPlugins` app with a
//!   real window + a real loaded font) so it can assert on ACTUAL computed pixel geometry against
//!   the LONGEST shipped ranged weapon name (read from the real `.ron` files, not hardcoded).

mod aim_stance_layout;
mod bottom_bar;
mod harness;
mod layout_geometry;
mod panel_content;
mod real_layout_harness;
mod reload_button;
mod structure;
