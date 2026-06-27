//! The gang-editor scene's systems (GTW-420): model lifecycle, screen spawn, gang-name edit,
//! add-member.

mod model_lifecycle;
pub(in crate::states::running::editor) use model_lifecycle::{
    insert_editable_gang, remove_editable_gang,
};

mod spawn;
pub(in crate::states::running::editor) use spawn::spawn_editor_screen;

mod name_edit;
pub(in crate::states::running::editor) use name_edit::commit_gang_name;

mod add_member;
pub(in crate::states::running::editor) use add_member::add_member_on_press;
