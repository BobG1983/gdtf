//! Spawns the gang-editor screen on `OnEnter(RunningState::DebugEditor)` (GTW-420 scaffold +
//! GTW-425 collapsed rows + GTW-428 expanded stat table).
//!
//! Builds a themed panel layout holding:
//!
//! - a **title** heading,
//! - the gang-NAME text field via [`spawn_text_field`](gdtf_ui::spawn_text_field) (carrying the
//!   [`GangNameField`] marker so its commit maps to the model name — AC3),
//! - an **"Add member"** button via [`spawn_button`](gdtf_ui::spawn_button) (the
//!   [`AddMemberButton`] marker — AC4),
//! - the member-list SHELL: a GTW-412 [`ScrollList`](gdtf_ui::ScrollList) ([`MemberListHost`](crate::states::running::editor::components::MemberListHost) on
//!   its root frame), seeded with one COLLAPSED [`MemberRow`](crate::states::running::editor::components::MemberRow) per member already in the model.
//!
//! Each member ROW is a flex column: a collapsed HEADER row above the GTW-428 expandable
//! [`MemberStatPanel`](crate::states::running::editor::components::MemberStatPanel). The header row (GTW-425 C1) contains, left→right: an [`ExpandPip`](crate::states::running::editor::components::ExpandPip) `+`/`-`
//! toggle, a [`MemberPortrait`](crate::states::running::editor::components::MemberPortrait) placeholder, the inline [`MemberNameField`](crate::states::running::editor::components::MemberNameField) beside a
//! [`MemberNameText`](crate::states::running::editor::components::MemberNameText) echo, a [`MemberWeaponText`](crate::states::running::editor::components::MemberWeaponText) + a [`MemberWeaponDropdown`](crate::states::running::editor::components::MemberWeaponDropdown) over all loaded
//! [`WeaponName`](gdtf_battle_sim::WeaponName) keys, a [`MemberArmorText`](crate::states::running::editor::components::MemberArmorText) + a
//! [`MemberArmorDropdown`](crate::states::running::editor::components::MemberArmorDropdown) over all loaded [`ArmorName`](gdtf_battle_sim::ArmorName) keys, and a
//! [`DeleteMemberButton`](crate::states::running::editor::components::DeleteMemberButton). BELOW it the GTW-428 stat panel lerps open on a pip press to show the
//! eight editable [`AttributeField`](crate::states::running::editor::components::AttributeField) numeric fields and the eight readonly [`DerivedStatText`](crate::states::running::editor::components::DerivedStatText)
//! displays in TWO COLUMNS — the panel animates open to a per-instance height waypoint then settles
//! to a CONTENT-FIT ([`Val::Auto`](bevy::ui::Val)) height (the GTW-428 layout fix, opting the panel
//! into [`AccordionContentFit`](gdtf_ui::AccordionContentFit)) so every line is visible regardless
//! of font metrics rather than clipped at the shared 18vh accordion default.
//!
//! Every entity carries [`DespawnOnExit(RunningState::DebugEditor)`](bevy::prelude::DespawnOnExit)
//! so the whole screen tears down on leave (C1). The model resource is inserted by a sibling
//! system ([`insert_editable_gang`](super::model_lifecycle::insert_editable_gang)) ordered before
//! this one, so the member count is known when the shell is seeded.
//!
//! ## Module layout
//!
//! | Submodule | Concern |
//! |-----------|---------|
//! | [`shell`] | `spawn_editor_screen` — full editor screen root + panel + list host |
//! | [`row`]   | `spawn_member_row` — collapsed header row + pip/portrait/name/delete |
//! | [`stat_panel`] | `spawn_member_stat_panel` — expanded accordion stat panel + attribute/derived fields |
//! | [`loadout`] | `spawn_weapon_loadout` / `spawn_armor_loadout` + sorted option list builders |

mod loadout;
mod row;
mod shell;
mod stat_panel;

// Public surface consumed by systems::mod.rs re-exports (via `pub(in crate::states::running::editor)` paths).
pub(in crate::states::running::editor) use loadout::{sorted_armor_options, sorted_weapon_options};
pub(in crate::states::running::editor) use row::spawn_member_row;
pub(in crate::states::running::editor) use shell::spawn_editor_screen;
