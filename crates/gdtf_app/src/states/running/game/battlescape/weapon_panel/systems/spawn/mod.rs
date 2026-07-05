//! Spawns + despawns the battlescape weapon cluster (GTW-275 / GTW-295 / GTW-298, bottom-left).
//!
//! [`spawn_weapon_panel`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds a themed
//! [`gdtf_ui`] panel group on the GTW-120 UI camera to the AUTHORITATIVE layout (GTW-298, user
//! 2026-06-18): the [`WeaponPanelRoot`](super::super::components::WeaponPanelRoot) **Overall Weapon Panel** box anchored BOTTOM-LEFT and
//! sitting IN the bottom bar (its height = the bar height, drawn one z ABOVE the bar — GTW-275
//! layout overhaul item 6: no bleed out the top), laid out as a 2×2 grid (every size a RELATIVE
//! unit — `Percent` of parent / `Vw` / `Vh` / flex — no fixed px but the hairline border):
//!
//! - **LEFT column** (3/4 width): the [`CombinedWeaponPanel`](super::super::components::CombinedWeaponPanel) (top 3/4 height) over the Firemode
//!   Panel (bottom 1/4 height);
//! - **RIGHT column** (1/4 width): the [`WeaponItemPanel`](super::super::components::WeaponItemPanel) (top 3/4 height) over the [`AimPanel`](super::super::components::AimPanel)
//!   (bottom 1/4 height).
//!
//! The **Combined Weapon Panel** is ONE bordered box: a FULL-WIDTH [`WeaponImage`](super::super::components::WeaponImage) placeholder
//! (top 1/2 height) over an info row of [the [`WeaponContent`](super::super::components::WeaponContent) weapon-text column (name +
//! magazine, the FLEX SPONGE) | the LIVE [`ReloadButton`](super::super::components::ReloadButton) (pinned right)] (bottom 1/2 height).
//! The **Item Panel** holds two stacked DISABLED [`WeaponItemButton`](super::super::components::WeaponItemButton)s (1/2 height each, full
//! width). The **Firemode / Aim** panels host the controls RELOCATED from the action bar
//! (GTW-298): the firemode 3-toggle [`spawn_mode_panel`](crate::states::running::game::battlescape::action_bar::spawn_mode_panel) column fills the Firemode cell, and the
//! [`AimToggleButton`](crate::states::running::game::battlescape::action_bar::components::AimToggleButton) fills the [`AimPanel`](super::super::components::AimPanel).
//!
//! A SEPARATE **Stance Panel** ([`spawn_stance_panel`](crate::states::running::game::battlescape::action_bar::spawn_stance_panel)) sits to the RIGHT of the Overall Weapon
//! Panel — its OWN bordered `Themed(Panel)` box (D-B, screenshot review 2026-06-18), sized to the
//! SAME HEIGHT as the Overall Weapon Panel ([`PANEL_H_VH`](geometry::PANEL_H_VH), NOT the full bottom-bar height) at a
//! fixed relative width ([`STANCE_W_VW`](geometry::STANCE_W_VW)), wrapping the three relocated stance toggles. It is
//! parented as a CHILD of the [`BottomBarRoot`](crate::states::running::game::battlescape::bottom_bar::BottomBarRoot) container — so it is laid out INSIDE the bottom
//! panel, contained by the bar's bounds + stacking context, NOT a free-floating overlay sitting ON
//! TOP of the bottom panel's edge. Its fixed-% width keeps it from changing the Overall panel's own
//! width (item 7 — the map area is stable). The relocated controls keep their action-bar markers,
//! so the existing press → intent router + active-mark syncs drive them unchanged.
//!
//! Sizing is RESPONSIVE (GTW-295): the root carries window-relative [`Val::Vw`](bevy::ui::Val)
//! width + [`Val::Vh`](bevy::ui::Val) height, the columns / cells split it by `Percent`, and the
//! weapon-text column carries a `min_height` so it cannot collapse to the empty-text minimum.
//! The weapon-text column is HIDDEN via [`Display::None`](bevy::ui::Display::None) when there is no selection / no weapon,
//! revealed + repainted from the selection by
//! [`update_weapon_panel`](super::update::update_weapon_panel) every battle frame, mutating the
//! existing widgets ([[ui-mutate-not-respawn]]).
//!
//! [`despawn_weapon_panel`] runs `OnExit(BattleScapeState::BattleRunning)` and recursively
//! despawns the whole cluster by its [`WeaponPanelRoot`](super::super::components::WeaponPanelRoot) + Stance-panel markers — battle-scoped,
//! mirroring the sibling status panel / action bar.

mod columns;
mod combined;
mod geometry;
mod item_aim_panels;
mod root;

pub(in crate::states::running::game::battlescape) use root::{
    despawn_weapon_panel, spawn_weapon_panel,
};
