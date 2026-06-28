//! The gang-editor scene's systems (GTW-420 scaffold + GTW-425 collapsed rows + GTW-428 expanded
//! per-member stat table): model lifecycle, screen spawn, gang-name edit, add-member, per-member
//! inline edit (name / weapon / armor), delete, the expand-pip→accordion toggle, and the
//! base-attribute edit with its live derived-stat recompute.

mod model_lifecycle;
pub(in crate::states::running::editor) use model_lifecycle::{
    insert_editable_gang, remove_editable_gang,
};

pub(in crate::states::running::editor) mod spawn;
pub(in crate::states::running::editor) use spawn::spawn_editor_screen;

/// The shared derived-stat formatter (GTW-428 C3) — the single rendering of a
/// [`DerivedStats`](gdtf_battle_sim::DerivedStats) field, used by BOTH the spawn seed and the live
/// recompute so they can never drift.
pub(in crate::states::running::editor) mod derived_display;

mod name_edit;
pub(in crate::states::running::editor) use name_edit::commit_gang_name;

mod add_member;
pub(in crate::states::running::editor) use add_member::add_member_on_press;

mod member_edit;
pub(in crate::states::running::editor) use member_edit::{
    commit_member_armor, commit_member_name, commit_member_weapon,
};

mod attribute_edit;
pub(in crate::states::running::editor) use attribute_edit::commit_member_attribute;

mod delete_member;
pub(in crate::states::running::editor) use delete_member::delete_member_on_press;

mod pip_toggle;
pub(in crate::states::running::editor) use pip_toggle::toggle_expand_pip;

// The GTW-429 gang SAVE-to-disk system + its serialize helpers. `#[cfg(debug_assertions)]`-gated
// (C3): the filesystem write is never compiled into a release binary. The round-trip test names
// the serialize / path helpers through this module.
#[cfg(debug_assertions)]
pub(in crate::states::running::editor) mod save;
#[cfg(debug_assertions)]
pub(in crate::states::running::editor) use save::save_gang_on_press;
