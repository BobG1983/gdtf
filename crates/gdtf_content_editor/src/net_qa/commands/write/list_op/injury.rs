//! The Injury draft's effects list: add and remove, keeping at least one row.

use gdtf_qa_protocol::command::RefusalNote;

use crate::{
    injury_form::InjuryDraft,
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{EditorListMemberNet, EditorListOpNet, InjuryEffectNet},
    },
};

const KEEPS_ONE: RefusalNote = RefusalNote::from_static(
    "an injury keeps at least one effect, so the form disables its own Remove button while one \
     remains",
);

/// The effects the draft holds, as the reply reads them back.
pub(super) fn members(draft: &InjuryDraft) -> Vec<EditorListMemberNet> {
    draft
        .effects()
        .iter()
        .map(|effect| EditorListMemberNet::InjuryEffect(InjuryEffectNet::from_effect(*effect)))
        .collect()
}

fn remove(draft: &mut InjuryDraft, index: usize) -> Result<(), FormWriteFault> {
    if !draft.can_remove_effect() {
        return Err(FormWriteFault::Gated(KEEPS_ONE));
    }
    let held = draft.effects().len();
    if index >= held {
        return Err(FormWriteFault::bad(format!(
            "effect {index} is past the end of a list holding {held}"
        )));
    }
    draft.remove_effect(index);
    Ok(())
}

/// Apply one operation to the injury's effect list.
pub(super) fn apply(draft: &mut InjuryDraft, op: EditorListOpNet) -> Result<(), FormWriteFault> {
    match op {
        EditorListOpNet::Add => {
            draft.add_effect();
            Ok(())
        }
        EditorListOpNet::Remove(index) => remove(draft, *index),
        EditorListOpNet::Toggle(_) => Err(FormWriteFault::bad(
            "the injury's effect list is authored by adding and removing rows, so it offers no \
             toggle"
                .to_owned(),
        )),
        EditorListOpNet::SetAt(..) => Err(FormWriteFault::bad(
            "one injury effect is rewritten through the `Injury(Effect(index: n, effect: …))` \
             field arm, not through the list"
                .to_owned(),
        )),
        EditorListOpNet::MoveUp(_) | EditorListOpNet::MoveDown(_) => Err(FormWriteFault::bad(
            "the injury's effect list draws no reorder buttons. The Sprite form's animation \
             frames and the two on-death effect lists are the lists that reorder"
                .to_owned(),
        )),
    }
}
