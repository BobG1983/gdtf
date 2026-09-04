//! The `editor.set_field` command itself: one field of the open form's draft.

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use serde::{Deserialize, Serialize};

use super::{route, terrain::TerrainWriteRegistries};
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
        wire::{EditorFieldNet, EditorModeNet},
    },
};

const NO_DRAFTS: RefusalNote = RefusalNote::from_static(
    "editor.set_field writes a form's draft, and every draft is a resource the editor only \
     creates on entering Editing",
);

const NOT_A_FORM_TAB: RefusalNote = RefusalNote::from_static(
    "editor.set_field writes the open form's draft, so it needs a form tab. The Prefab tab is \
     the map canvas and holds no draft",
);

const DRAFT_GONE: RefusalNote =
    RefusalNote::from_static("the open form's own draft resource is not in the world");

const NO_MODE_RESOURCE: RefusalNote = RefusalNote::from_static(
    "the mode tab resource left the world between the availability check and the handler",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorSetFieldArgs {
    field: EditorFieldNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorSetFieldReply {
    mode:  EditorModeNet,
    field: EditorFieldNet,
}

pub(in crate::mcp) struct EditorSetField;

impl QaCommand for EditorSetField {
    type Args = EditorSetFieldArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorSetFieldReply;

    const NAME: CommandName = CommandName::from_static("editor.set_field");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Write one single-value field of the open form's draft, the way that form's own widget \
         writes it. Each field arm names the form it belongs to, so an arm from another form is \
         refused rather than written.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_in_a_form_mode_with_its_draft(*facts, NO_DRAFTS, NOT_A_FORM_TAB, DRAFT_GONE)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_set_field
                .after(QaCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

/// What a field write answers: the field as the draft stores it, or why it did not land.
pub(super) type Written = Result<EditorFieldNet, SetFieldRefusal>;

/// Why a write did not land: the draft went missing, or the form turned the write down.
pub(super) enum SetFieldRefusal {
    /// The open tab's own draft resource is not in the world.
    DraftGone,
    /// The form read the write and turned it down.
    Fault(FormWriteFault),
}

impl From<FormWriteFault> for SetFieldRefusal {
    fn from(fault: FormWriteFault) -> Self {
        Self::Fault(fault)
    }
}

/// The draft the open tab names, or the refusal a draft that left the world answers.
pub(super) fn present<T>(draft: Option<&mut T>) -> Result<&mut T, SetFieldRefusal> {
    draft.ok_or(SetFieldRefusal::DraftGone)
}

/// A field of a form other than the open tab, which answers for its own missing draft first.
pub(super) fn no_field_of_its_own<T>(draft: Option<&mut T>) -> Written {
    present(draft)?;
    Err(SetFieldRefusal::Fault(FormWriteFault::ForeignArm))
}

fn write_to(
    forms: &mut EditorForms,
    registries: &EditorRegistries,
    mode: EditorModeNet,
    field: EditorFieldNet,
) -> Written {
    match mode {
        EditorModeNet::Terrain => route::terrain(
            forms.terrain.as_mut(),
            &TerrainWriteRegistries {
                weapons: registries.weapons.as_deref(),
                terrain: registries.terrain.as_deref(),
                sprites: registries.sprites.as_deref(),
            },
            field,
        ),
        EditorModeNet::Theme => route::theme(forms.theme.as_mut(), field),
        EditorModeNet::Prefab => route::prefab(field),
        EditorModeNet::Gang => route::gang(
            forms.gang.as_mut(),
            registries.weapons.as_deref(),
            registries.melee_weapon.as_deref(),
            registries.armor.as_deref(),
            field,
        ),
        EditorModeNet::Armor => route::armor(forms.armor.as_mut(), field),
        EditorModeNet::Injury => route::injury(
            forms.injury.as_mut(),
            forms.weighting.as_mut(),
            registries.injuries.as_deref(),
            field,
        ),
        EditorModeNet::Sprite => route::sprite(forms.sprite.as_mut(), field),
        EditorModeNet::Attachment => route::attachment(forms.attachment.as_mut(), field),
        EditorModeNet::Weapon => route::weapon(forms.weapon.as_mut(), field),
        EditorModeNet::MeleeWeapon => route::melee_weapon(forms.melee_weapon.as_mut(), field),
        EditorModeNet::Field => route::field(forms.field.as_mut(), field),
    }
}

fn handle_editor_set_field(
    mode: Option<Res<EditorMode>>,
    mut forms: EditorForms,
    registries: EditorRegistries,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSetField>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(mode) = mode.as_deref().copied().map(EditorModeNet::from_mode) else {
        for (_args, responder) in take_calls::<EditorSetField>(&mut queue) {
            responder.unavailable(UnavailableCode::WrongState, NO_MODE_RESOURCE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorSetField>(&mut queue) {
        match write_to(&mut forms, &registries, mode, args.field) {
            Ok(field) => responder.answer(&EditorSetFieldReply { mode, field }),
            Err(SetFieldRefusal::DraftGone) => responder.unavailable(
                UnavailableCode::WrongState,
                RefusalNote::from_owned(format!("the {mode:?} draft resource is not in the world")),
            ),
            Err(SetFieldRefusal::Fault(FormWriteFault::ForeignArm)) => {
                responder.unavailable(UnavailableCode::WrongState, foreign_arm_note(mode));
            }
            Err(SetFieldRefusal::Fault(FormWriteFault::Gated(note))) => {
                responder.unavailable(UnavailableCode::WrongState, note);
            }
            Err(SetFieldRefusal::Fault(FormWriteFault::MissingModel(note))) => {
                responder.unavailable(UnavailableCode::MissingModel, note);
            }
            Err(SetFieldRefusal::Fault(FormWriteFault::BadArguments(detail))) => {
                responder.bad_arguments(detail);
            }
        }
    }
}
