//! The `editor.set_field` command itself: one field of the open form's draft.

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

use super::{
    armor, attachment, field, gang, injury, melee_weapon, sprite, terrain, weapon, weighting,
};
use crate::{
    EditorMode,
    net_qa::{
        commands::{
            availability::only_in_a_form_mode_with_its_draft,
            write::form_fault::{FormWriteFault, foreign_arm_note},
        },
        facts::EditorFacts,
        forms::{EditorForms, EditorRegistries},
        schedule::EditorNetQaSystems,
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
pub(in crate::net_qa) struct EditorSetFieldArgs {
    field: EditorFieldNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorSetFieldReply {
    mode:  EditorModeNet,
    field: EditorFieldNet,
}

pub(in crate::net_qa) struct EditorSetField;

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
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

// Why a write did not land: the draft went missing, or the form turned the write down.
enum SetFieldRefusal {
    DraftGone,
    Fault(FormWriteFault),
}

impl From<FormWriteFault> for SetFieldRefusal {
    fn from(fault: FormWriteFault) -> Self {
        Self::Fault(fault)
    }
}

// The draft the open tab names, or the refusal a draft that left the world answers.
fn present<T>(draft: Option<&mut T>) -> Result<&mut T, SetFieldRefusal> {
    draft.ok_or(SetFieldRefusal::DraftGone)
}

// Whether the open tab's own draft resource is still in the world.
const fn draft_in_world(forms: &EditorForms, mode: EditorModeNet) -> bool {
    match mode {
        EditorModeNet::Terrain => forms.terrain.is_some(),
        EditorModeNet::Theme => forms.theme.is_some(),
        EditorModeNet::Gang => forms.gang.is_some(),
        EditorModeNet::Armor => forms.armor.is_some(),
        EditorModeNet::Injury => forms.injury.is_some(),
        EditorModeNet::Sprite => forms.sprite.is_some(),
        EditorModeNet::Attachment => forms.attachment.is_some(),
        EditorModeNet::Weapon => forms.weapon.is_some(),
        EditorModeNet::MeleeWeapon => forms.melee_weapon.is_some(),
        EditorModeNet::Field => forms.field.is_some(),
        EditorModeNet::Prefab => true,
    }
}

// A field of a form other than the open tab, which answers for its own missing draft first.
const fn foreign_arm(
    forms: &EditorForms,
    mode: EditorModeNet,
) -> Result<EditorFieldNet, SetFieldRefusal> {
    if draft_in_world(forms, mode) {
        Err(SetFieldRefusal::Fault(FormWriteFault::ForeignArm))
    } else {
        Err(SetFieldRefusal::DraftGone)
    }
}

fn write_to(
    forms: &mut EditorForms,
    registries: &EditorRegistries,
    mode: EditorModeNet,
    field: EditorFieldNet,
) -> Result<EditorFieldNet, SetFieldRefusal> {
    match (mode, field) {
        (EditorModeNet::Terrain, EditorFieldNet::Terrain(field)) => {
            Ok(EditorFieldNet::Terrain(terrain::write(
                present(forms.terrain.as_mut())?,
                registries.weapons.as_deref(),
                field,
            )?))
        }
        (EditorModeNet::Armor, EditorFieldNet::Armor(field)) => Ok(EditorFieldNet::Armor(
            armor::write(present(forms.armor.as_mut())?, field)?,
        )),
        (EditorModeNet::Sprite, EditorFieldNet::Sprite(field)) => Ok(EditorFieldNet::Sprite(
            sprite::write(present(forms.sprite.as_mut())?, field)?,
        )),
        (EditorModeNet::Attachment, EditorFieldNet::Attachment(field)) => {
            Ok(EditorFieldNet::Attachment(attachment::write(
                present(forms.attachment.as_mut())?,
                field,
            )?))
        }
        (EditorModeNet::Injury, EditorFieldNet::Injury(field)) => Ok(EditorFieldNet::Injury(
            injury::write(present(forms.injury.as_mut())?, field)?,
        )),
        (EditorModeNet::Injury, EditorFieldNet::Weighting(field)) => {
            Ok(EditorFieldNet::Weighting(weighting::write(
                present(forms.weighting.as_mut())?,
                registries.injuries.as_deref(),
                field,
            )?))
        }
        (EditorModeNet::MeleeWeapon, EditorFieldNet::MeleeWeapon(field)) => {
            Ok(EditorFieldNet::MeleeWeapon(melee_weapon::write(
                present(forms.melee_weapon.as_mut())?,
                field,
            )))
        }
        (EditorModeNet::Gang, EditorFieldNet::Gang(field)) => {
            Ok(EditorFieldNet::Gang(gang::write(
                present(forms.gang.as_mut())?,
                registries.weapons.as_deref(),
                registries.melee_weapon.as_deref(),
                registries.armor.as_deref(),
                field,
            )?))
        }
        (EditorModeNet::Weapon, EditorFieldNet::Weapon(field)) => Ok(EditorFieldNet::Weapon(
            weapon::write(present(forms.weapon.as_mut())?, field)?,
        )),
        (EditorModeNet::Field, EditorFieldNet::Field(field)) => Ok(EditorFieldNet::Field(
            field::write(present(forms.field.as_mut())?, field)?,
        )),
        (mode, _) => foreign_arm(forms, mode),
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
