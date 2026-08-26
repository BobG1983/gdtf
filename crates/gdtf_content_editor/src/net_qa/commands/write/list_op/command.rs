//! The `editor.list_op` command itself: one list of the open form's draft.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use super::{attachment, sprite, terrain};
use crate::{
    EditorMode,
    net_qa::{
        commands::{
            availability::only_in_a_form_mode_with_its_draft,
            write::form_fault::{FormWriteFault, foreign_arm_note},
        },
        facts::EditorFacts,
        forms::EditorForms,
        schedule::EditorNetQaSystems,
        wire::{EditorListMemberNet, EditorListNet, EditorListOpNet, EditorModeNet},
    },
};

const NO_DRAFTS: RefusalNote = RefusalNote::from_static(
    "editor.list_op edits a form's draft, and every draft is a resource the editor only creates \
     on entering Editing",
);

const NOT_A_FORM_TAB: RefusalNote = RefusalNote::from_static(
    "editor.list_op edits the open form's draft, so it needs a form tab. The Prefab tab is the \
     map canvas and holds no draft",
);

const DRAFT_GONE: RefusalNote =
    RefusalNote::from_static("the open form's own draft resource is not in the world");

const NO_MODE_RESOURCE: RefusalNote = RefusalNote::from_static(
    "the mode tab resource left the world between the availability check and the handler",
);

const ARMOR_HAS_NO_LIST: &str = "the Armor form draws six fixed pieces and no list at all, so no list name is right while \
     its tab is open";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorListOpArgs {
    list: EditorListNet,
    op:   EditorListOpNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorListOpReply {
    mode:    EditorModeNet,
    list:    EditorListNet,
    members: Vec<EditorListMemberNet>,
}

pub(in crate::net_qa) struct EditorListOp;

impl QaCommand for EditorListOp {
    type Args = EditorListOpArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorListOpReply;

    const NAME: CommandName = CommandName::from_static("editor.list_op");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Edit one list-valued field of the open form's draft through that form's own setter. \
         Each list name belongs to one form, so a list from another form is refused, and an \
         operation the named list does not draw answers BadArguments.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_in_a_form_mode_with_its_draft(*facts, NO_DRAFTS, NOT_A_FORM_TAB, DRAFT_GONE)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_list_op
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

// Why an edit did not land: the draft went missing, or the form turned the edit down.
enum ListOpRefusal {
    DraftGone,
    Fault(FormWriteFault),
}

impl From<FormWriteFault> for ListOpRefusal {
    fn from(fault: FormWriteFault) -> Self {
        Self::Fault(fault)
    }
}

// The draft the open tab names, or the refusal a draft that left the world answers.
fn present<T>(draft: Option<&mut T>) -> Result<&mut T, ListOpRefusal> {
    draft.ok_or(ListOpRefusal::DraftGone)
}

// A form with no list of its own still answers for its own missing draft first.
fn no_list_of_its_own<T>(draft: Option<&mut T>) -> Result<Vec<EditorListMemberNet>, ListOpRefusal> {
    present(draft)?;
    Err(ListOpRefusal::Fault(FormWriteFault::ForeignArm))
}

fn edit(
    forms: &mut EditorForms,
    mode: EditorModeNet,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Result<Vec<EditorListMemberNet>, ListOpRefusal> {
    match (mode, list) {
        (EditorModeNet::Armor, _) => {
            present(forms.armor.as_mut())?;
            Err(ListOpRefusal::Fault(FormWriteFault::bad(
                ARMOR_HAS_NO_LIST.to_owned(),
            )))
        }
        (EditorModeNet::Terrain, EditorListNet::EntrySides) => {
            let draft = present(forms.terrain.as_mut())?;
            terrain::apply(draft, op)?;
            Ok(terrain::members(draft))
        }
        (EditorModeNet::Attachment, EditorListNet::AttachmentEffects) => {
            let draft = present(forms.attachment.as_mut())?;
            attachment::apply(draft, op)?;
            Ok(attachment::members(draft))
        }
        (EditorModeNet::Sprite, EditorListNet::SpriteFrames) => {
            let draft = present(forms.sprite.as_mut())?;
            sprite::apply(draft, op)?;
            Ok(sprite::members(draft))
        }
        (EditorModeNet::Terrain, _) => no_list_of_its_own(forms.terrain.as_mut()),
        (EditorModeNet::Attachment, _) => no_list_of_its_own(forms.attachment.as_mut()),
        (EditorModeNet::Sprite, _) => no_list_of_its_own(forms.sprite.as_mut()),
        (EditorModeNet::Theme, _) => no_list_of_its_own(forms.theme.as_mut()),
        (EditorModeNet::Gang, _) => no_list_of_its_own(forms.gang.as_mut()),
        (EditorModeNet::Injury, _) => no_list_of_its_own(forms.injury.as_mut()),
        (EditorModeNet::Weapon, _) => no_list_of_its_own(forms.weapon.as_mut()),
        (EditorModeNet::MeleeWeapon, _) => no_list_of_its_own(forms.melee_weapon.as_mut()),
        (EditorModeNet::Prefab, _) => Err(ListOpRefusal::Fault(FormWriteFault::ForeignArm)),
    }
}

fn handle_editor_list_op(
    mode: Option<Res<EditorMode>>,
    mut forms: EditorForms,
    mut queue: ResMut<PendingQueue<CommandCall<EditorListOp>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(mode) = mode.as_deref().copied().map(EditorModeNet::from_mode) else {
        for (_args, responder) in take_calls::<EditorListOp>(&mut queue) {
            responder.unavailable(UnavailableCode::WrongState, NO_MODE_RESOURCE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorListOp>(&mut queue) {
        match edit(&mut forms, mode, args.list, args.op) {
            Ok(members) => responder.answer(&EditorListOpReply {
                mode,
                list: args.list,
                members,
            }),
            Err(ListOpRefusal::DraftGone) => responder.unavailable(
                UnavailableCode::MissingModel,
                RefusalNote::from_owned(format!("the {mode:?} draft resource is not in the world")),
            ),
            Err(ListOpRefusal::Fault(FormWriteFault::ForeignArm)) => {
                responder.unavailable(UnavailableCode::WrongState, foreign_arm_note(mode));
            }
            Err(ListOpRefusal::Fault(FormWriteFault::Gated(note))) => {
                responder.unavailable(UnavailableCode::WrongState, note);
            }
            Err(ListOpRefusal::Fault(FormWriteFault::BadArguments(detail))) => {
                responder.bad_arguments(detail);
            }
        }
    }
}
