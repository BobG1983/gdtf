//! The Injury form's own field arms, written through the draft its panels write.

use gdtf_battle_sim::injuries::{InjuryName, InspectText, LogText, PopupText};

use crate::{
    injury_form::InjuryDraft,
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{
            EditorDraftNameNet, EditorFieldNet, EditorListIndexNet, InjuryCategoryNet,
            InjuryEffectNet, InjuryKeyNet, InjurySeverityNet, InjuryTextNet,
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
) -> Result<EditorFieldNet, FormWriteFault> {
    effect_at(draft, index)?;
    draft.set_effect(index, effect.to_effect());
    Ok(EditorFieldNet::InjuryEffect {
        index:  EditorListIndexNet::new(index),
        effect: effect_at(draft, index)?,
    })
}

/// Write one Injury field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut InjuryDraft,
    field: EditorFieldNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::InjuryKey(key) => {
            draft.set_key((*key).clone());
            Ok(EditorFieldNet::InjuryKey(InjuryKeyNet::new(draft.key())))
        }
        EditorFieldNet::InjuryName(name) => {
            draft.def_mut().name = InjuryName::new((*name).clone());
            Ok(EditorFieldNet::InjuryName(EditorDraftNameNet::new(
                draft.def().name.as_str(),
            )))
        }
        EditorFieldNet::InjuryCategory(category) => {
            draft.def_mut().category = category.to_category();
            Ok(EditorFieldNet::InjuryCategory(
                InjuryCategoryNet::from_category(draft.def().category),
            ))
        }
        EditorFieldNet::InjurySeverity(severity) => {
            draft.def_mut().severity = severity.to_severity();
            Ok(EditorFieldNet::InjurySeverity(stored_severity(
                draft, severity,
            )))
        }
        EditorFieldNet::InjuryPopupText(text) => {
            draft.def_mut().popup_text = PopupText::new((*text).clone());
            Ok(EditorFieldNet::InjuryPopupText(InjuryTextNet::new(
                &draft.def().popup_text,
            )))
        }
        EditorFieldNet::InjuryLogText(text) => {
            draft.def_mut().log_text = LogText::new((*text).clone());
            Ok(EditorFieldNet::InjuryLogText(InjuryTextNet::new(
                &draft.def().log_text,
            )))
        }
        EditorFieldNet::InjuryInspectText(text) => {
            draft.def_mut().inspect_text = InspectText::new((*text).clone());
            Ok(EditorFieldNet::InjuryInspectText(InjuryTextNet::new(
                &draft.def().inspect_text,
            )))
        }
        EditorFieldNet::InjuryEffect { index, effect } => write_effect(draft, *index, effect),
        _ => Err(FormWriteFault::ForeignArm),
    }
}
