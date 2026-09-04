//! The `editor.list_op` command itself: one list of the open form's draft.

use bevy::prelude::*;
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use super::route;
use crate::{
    EditorMode,
    mcp::{
        commands::{
            availability::only_in_a_form_mode_with_its_draft,
            write::form_fault::{FormWriteFault, foreign_arm_note},
        },
        facts::EditorFacts,
        forms::{EditorForms, EditorRegistries},
        schedule::EditorMcpSystems,
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorListOpArgs {
    list: EditorListNet,
    op:   EditorListOpNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorListOpReply {
    mode:    EditorModeNet,
    list:    EditorListNet,
    members: Vec<EditorListMemberNet>,
}

pub(in crate::mcp) struct EditorListOp;

impl McpCommand for EditorListOp {
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
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

/// What a list op answers: the named list's members, or why the edit did not land.
pub(super) type Routed = Result<Vec<EditorListMemberNet>, ListOpRefusal>;

/// Why an edit did not land: the draft went missing, or the form turned the edit down.
pub(super) enum ListOpRefusal {
    /// The open tab's own draft resource is not in the world.
    DraftGone,
    /// The form read the edit and turned it down.
    Fault(FormWriteFault),
}

impl From<FormWriteFault> for ListOpRefusal {
    fn from(fault: FormWriteFault) -> Self {
        Self::Fault(fault)
    }
}

/// The draft the open tab names, or the refusal a draft that left the world answers.
pub(super) fn present<T>(draft: Option<&mut T>) -> Result<&mut T, ListOpRefusal> {
    draft.ok_or(ListOpRefusal::DraftGone)
}

/// A form with no list of its own still answers for its own missing draft first.
pub(super) fn no_list_of_its_own<T>(draft: Option<&mut T>) -> Routed {
    present(draft)?;
    Err(ListOpRefusal::Fault(FormWriteFault::ForeignArm))
}

fn edit(
    forms: &mut EditorForms,
    registries: &EditorRegistries,
    mode: EditorModeNet,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Routed {
    match mode {
        EditorModeNet::Terrain => route::terrain(forms.terrain.as_mut(), list, op),
        EditorModeNet::Theme => route::theme(forms.theme.as_mut(), list),
        EditorModeNet::Prefab => route::prefab(list),
        EditorModeNet::Gang => route::gang(forms.gang.as_mut(), list, op),
        EditorModeNet::Armor => route::armor(forms.armor.as_mut(), list),
        EditorModeNet::Injury => route::injury(
            forms.injury.as_mut(),
            forms.weighting.as_mut(),
            registries.injuries.as_deref(),
            list,
            op,
        ),
        EditorModeNet::Sprite => route::sprite(forms.sprite.as_mut(), list, op),
        EditorModeNet::Attachment => route::attachment(forms.attachment.as_mut(), list, op),
        EditorModeNet::Weapon => route::weapon(
            forms.weapon.as_mut(),
            registries.attachments.as_deref(),
            list,
            op,
        ),
        EditorModeNet::MeleeWeapon => route::melee_weapon(
            forms.melee_weapon.as_mut(),
            registries.attachments.as_deref(),
            list,
            op,
        ),
        EditorModeNet::Field => route::field(forms.field.as_mut(), list, op),
    }
}

fn handle_editor_list_op(
    mode: Option<Res<EditorMode>>,
    mut forms: EditorForms,
    registries: EditorRegistries,
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
        match edit(&mut forms, &registries, mode, args.list, args.op) {
            Ok(members) => responder.answer(&EditorListOpReply {
                mode,
                list: args.list,
                members,
            }),
            Err(ListOpRefusal::DraftGone) => responder.unavailable(
                UnavailableCode::WrongState,
                RefusalNote::from_owned(format!("the {mode:?} draft resource is not in the world")),
            ),
            Err(ListOpRefusal::Fault(FormWriteFault::ForeignArm)) => {
                responder.unavailable(UnavailableCode::WrongState, foreign_arm_note(mode));
            }
            Err(ListOpRefusal::Fault(FormWriteFault::Gated(note))) => {
                responder.unavailable(UnavailableCode::WrongState, note);
            }
            Err(ListOpRefusal::Fault(FormWriteFault::MissingModel(note))) => {
                responder.unavailable(UnavailableCode::MissingModel, note);
            }
            Err(ListOpRefusal::Fault(FormWriteFault::BadArguments(detail))) => {
                responder.bad_arguments(detail);
            }
        }
    }
}
