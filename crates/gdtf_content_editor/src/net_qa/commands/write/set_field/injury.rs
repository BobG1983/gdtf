//! The Injury form's own field arms, written through the draft its panels write.

use gdtf_battle_sim::injuries::{InjuryName, InspectText, LogText, PopupText};

use crate::{
    injury_form::InjuryDraft,
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{
            EditorDraftNameNet, EditorListIndexNet, InjuryCategoryNet, InjuryEffectNet,
            InjuryFieldNet, InjuryKeyNet, InjurySeverityNet, InjuryTextNet,
        },
    },
};

// The effect stored at one index, or the fault a past-the-end index answers.
fn effect_at(draft: &InjuryDraft, index: usize) -> Result<InjuryEffectNet, FormWriteFault> {
    match draft.effects().get(index) {
        Some(effect) => Ok(InjuryEffectNet::from_effect(*effect)),
        None => Err(FormWriteFault::bad(format!(
            "effect {index} is past the end of the injury's effect list"
        ))),
    }
}

// The rank the draft stores, which the form's own combo can only ever leave tabled.
fn stored_severity(draft: &InjuryDraft, sent: InjurySeverityNet) -> InjurySeverityNet {
    InjurySeverityNet::from_severity(draft.def().severity).unwrap_or(sent)
}

fn write_effect(
    draft: &mut InjuryDraft,
    index: usize,
    effect: InjuryEffectNet,
) -> Result<InjuryFieldNet, FormWriteFault> {
    effect_at(draft, index)?;
    draft.set_effect(index, effect.to_effect());
    Ok(InjuryFieldNet::Effect {
        index:  EditorListIndexNet::new(index),
        effect: effect_at(draft, index)?,
    })
}

/// Write one Injury field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut InjuryDraft,
    field: InjuryFieldNet,
) -> Result<InjuryFieldNet, FormWriteFault> {
    match field {
        InjuryFieldNet::Key(key) => {
            draft.set_key((*key).clone());
            Ok(InjuryFieldNet::Key(InjuryKeyNet::new(draft.key())))
        }
        InjuryFieldNet::Name(name) => {
            draft.def_mut().name = InjuryName::new((*name).clone());
            Ok(InjuryFieldNet::Name(EditorDraftNameNet::new(
                draft.def().name.as_str(),
            )))
        }
        InjuryFieldNet::Category(category) => {
            draft.def_mut().category = category.to_category();
            Ok(InjuryFieldNet::Category(InjuryCategoryNet::from_category(
                draft.def().category,
            )))
        }
        InjuryFieldNet::Severity(severity) => {
            draft.def_mut().severity = severity.to_severity();
            Ok(InjuryFieldNet::Severity(stored_severity(draft, severity)))
        }
        InjuryFieldNet::PopupText(text) => {
            draft.def_mut().popup_text = PopupText::new((*text).clone());
            Ok(InjuryFieldNet::PopupText(InjuryTextNet::new(
                &draft.def().popup_text,
            )))
        }
        InjuryFieldNet::LogText(text) => {
            draft.def_mut().log_text = LogText::new((*text).clone());
            Ok(InjuryFieldNet::LogText(InjuryTextNet::new(
                &draft.def().log_text,
            )))
        }
        InjuryFieldNet::InspectText(text) => {
            draft.def_mut().inspect_text = InspectText::new((*text).clone());
            Ok(InjuryFieldNet::InspectText(InjuryTextNet::new(
                &draft.def().inspect_text,
            )))
        }
        InjuryFieldNet::Effect { index, effect } => write_effect(draft, *index, effect),
    }
}
