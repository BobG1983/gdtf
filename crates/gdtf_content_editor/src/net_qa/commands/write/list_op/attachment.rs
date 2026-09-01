//! The Attachment draft's effects list: add and remove, and no reorder.

use crate::{
    attachment_form::AttachmentDraft,
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{AttachmentEffectNet, EditorListMemberNet, EditorListOpNet},
    },
};

/// The effects the draft holds, as the reply reads them back.
pub(super) fn members(draft: &AttachmentDraft) -> Vec<EditorListMemberNet> {
    draft
        .effects()
        .iter()
        .map(|effect| {
            EditorListMemberNet::AttachmentEffect(AttachmentEffectNet::from_effect(effect))
        })
        .collect()
}

/// Apply one operation to the effects list.
pub(super) fn apply(
    draft: &mut AttachmentDraft,
    op: EditorListOpNet,
) -> Result<(), FormWriteFault> {
    match op {
        EditorListOpNet::Add => {
            draft.add_effect();
            Ok(())
        }
        EditorListOpNet::Remove(index) => {
            if draft.remove_effect(*index) {
                Ok(())
            } else {
                Err(FormWriteFault::bad(format!(
                    "effect {} is past the end of the attachment's effect list",
                    *index
                )))
            }
        }
        EditorListOpNet::Toggle(_) => Err(FormWriteFault::bad(
            "the effects list is authored by adding and removing rows, so it offers no toggle"
                .to_owned(),
        )),
        EditorListOpNet::SetAt(..) => Err(FormWriteFault::bad(
            "one attachment effect is rewritten through the \
             `Attachment(Effect(index: n, effect: …))` field arm, not through the list"
                .to_owned(),
        )),
        EditorListOpNet::MoveUp(_) | EditorListOpNet::MoveDown(_) => Err(FormWriteFault::bad(
            "the effects list draws no reorder buttons. The Sprite form's animation frames and \
             the two on-death effect lists are the lists that reorder"
                .to_owned(),
        )),
    }
}
