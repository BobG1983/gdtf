//! The Gang draft's roster: append a default member, or take one off by index.

use crate::{
    gang_form::GangDraft,
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{EditorDraftNameNet, EditorListMemberNet, EditorListOpNet},
    },
};

// The line an operation the roster draws no button for is refused with.
fn no_such_button(op: &str) -> FormWriteFault {
    FormWriteFault::bad(format!(
        "the GangMembers list draws an Add button and a Remove button per row, so it takes no \
         {op}"
    ))
}

/// The members the draft holds, as the reply reads them back.
pub(in crate::net_qa::commands::write::list_op) fn members(
    draft: &GangDraft,
) -> Vec<EditorListMemberNet> {
    draft
        .members()
        .iter()
        .map(|member| {
            EditorListMemberNet::GangMember(EditorDraftNameNet::new(member.name.as_str()))
        })
        .collect()
}

// Remove one member, reading the roster's own answer for whether it held that row.
fn remove(draft: &mut GangDraft, index: usize) -> Result<(), FormWriteFault> {
    let held = draft.members().len();
    if draft.remove_member(index) {
        Ok(())
    } else {
        Err(FormWriteFault::bad(format!(
            "member {index} is past the end of a roster holding {held}"
        )))
    }
}

/// Apply one operation to the gang's member list.
pub(in crate::net_qa::commands::write::list_op) fn apply(
    draft: &mut GangDraft,
    op: EditorListOpNet,
) -> Result<(), FormWriteFault> {
    match op {
        EditorListOpNet::Add => {
            draft.add_member();
            Ok(())
        }
        EditorListOpNet::Remove(index) => remove(draft, *index),
        EditorListOpNet::Toggle(_) => Err(no_such_button("toggle")),
        EditorListOpNet::SetAt(..) => Err(no_such_button(
            "set-at. One member's own rows are rewritten through the `Gang(MemberName(…))`, \
             `Gang(MemberAttribute(…))`, `Gang(MemberWeapon(…))`, `Gang(MemberArmor(…))` and \
             `Gang(MemberMeleeWeapon(…))` field arms",
        )),
        EditorListOpNet::MoveUp(_) | EditorListOpNet::MoveDown(_) => Err(no_such_button(
            "reorder. The Sprite form's animation frames and the two on-death effect lists are \
             the lists that reorder",
        )),
    }
}
