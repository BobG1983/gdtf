//! The gang-editor scene's systems (GTW-420 scaffold + GTW-425 collapsed rows): model lifecycle,
//! screen spawn, gang-name edit, add-member, per-member inline edit (name / weapon / armor),
//! delete, and the expand-pip toggle.

mod model_lifecycle;
pub(in crate::states::running::editor) use model_lifecycle::{
    insert_editable_gang, remove_editable_gang,
};

pub(in crate::states::running::editor) mod spawn;
pub(in crate::states::running::editor) use spawn::spawn_editor_screen;

mod name_edit;
pub(in crate::states::running::editor) use name_edit::commit_gang_name;

mod add_member;
pub(in crate::states::running::editor) use add_member::add_member_on_press;

mod member_edit;
pub(in crate::states::running::editor) use member_edit::{
    commit_member_armor, commit_member_name, commit_member_weapon,
};

mod delete_member;
pub(in crate::states::running::editor) use delete_member::delete_member_on_press;

mod pip_toggle;
pub(in crate::states::running::editor) use pip_toggle::toggle_expand_pip;
