//! GTW-425: headless behavioral tests for the in-app gang-editor's COLLAPSED member rows + inline
//! editing.
//!
//! These run on the `MinimalPlugins` [`GdtfTestAppBuilder`] (the real state stack, `UiPlugin`, and
//! the real `GangEditorScenePlugin` wired through `ScenesPlugin`), seeded with a theme + a
//! [`WeaponRegistry`] / [`ArmorRegistry`] so the editor screen spawns with populated weapon /
//! armor dropdowns. They assert on the WORLD and the model resource (the system-effects) — never
//! on rendering or real device input.
//!
//! Per CRITICAL GOTCHA 2: editing is driven through the REAL input MESSAGES the widgets raise —
//! [`TextFieldCommitted`] / [`DropdownSelectionChanged`] — or the real action system via a set
//! [`Interaction::Pressed`] + a single `Update` run (NOT a full `app.update()`, which the windowed
//! `ui_focus_system` would clobber — but this harness is `MinimalPlugins`, so there is no
//! `ui_focus_system`; setting `Pressed` then `update()` is the accepted headless idiom, the
//! `gang_editor_scaffold.rs` precedent). Each assertion is pin-discriminating: it would FAIL if its
//! clause were reverted.
//!
//! Coverage (C6):
//!
//! - [`add_member_appears_inside_scroll_area`] — add a member → a row appears, parented INSIDE the
//!   member-list `ScrollListArea` (C1 + GOTCHA 1).
//! - [`member_row_has_exactly_one_control_per_field`] — a row carries EXACTLY ONE control per
//!   field (name / weapon / armor): the redundant doubled echo labels are gone (GTW-499 C1).
//! - [`commit_member_name_updates_model`] — a real `TextFieldCommitted` on a member's name
//!   field edits the model name (C3).
//! - [`commit_member_weapon_updates_model`] / [`commit_member_armor_updates_model`] — a real
//!   `DropdownSelectionChanged` edits the model loadout (C2).
//! - [`delete_member_removes_from_model_and_list`] — a delete press removes the member from the
//!   model AND despawns its row (C4).
//! - [`editing_one_member_leaves_other_rows_untouched`] — after editing ONE member, the OTHER
//!   rows' entity ids are unchanged (proves the edit MUTATES, never rebuilds the list — C5).

mod commits;
mod harness;
mod row_structure;
