//! `editor.load_attachment` fills the Attachment draft from the attachment registry.

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_sim::equipment::attachments::AttachmentRegistry;
use serde::{Deserialize, Serialize};

use super::super::families::{KeyLookup, load_attachment};
use crate::{
    attachment_form::AttachmentDraft,
    mcp::{
        commands::availability::only_on_the_tab,
        facts::EditorFacts,
        schedule::EditorMcpSystems,
        wire::{EditorKeyNet, EditorLoadOutcomeNet, EditorModeNet},
    },
};

const NO_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.load_attachment fills the Attachment draft, which the editor only creates on entering \
     Editing",
);

const NOT_THE_ATTACHMENT_TAB: RefusalNote = RefusalNote::from_static(
    "the attachment form's load picker is drawn only on the Attachment tab, so this load needs \
     that tab open",
);

const REGISTRY_GONE: RefusalNote =
    RefusalNote::from_static("the Attachment draft or the attachment registry is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorLoadAttachmentArgs {
    key: EditorKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorLoadAttachmentReply {
    outcome: EditorLoadOutcomeNet,
}

pub(in crate::mcp) struct EditorLoadAttachment;

impl QaCommand for EditorLoadAttachment {
    type Args = EditorLoadAttachmentArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorLoadAttachmentReply;

    const NAME: CommandName = CommandName::from_static("editor.load_attachment");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Load an attachment by key into the Attachment draft, through that draft's own \
         `load_attachment` method. A key the registry does not hold answers `NoSuchKey` with the \
         keys it does hold, and leaves the draft alone. Needs the Attachment tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_tab(
            *facts,
            EditorModeNet::Attachment,
            NO_DRAFT,
            NOT_THE_ATTACHMENT_TAB,
        )
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_load_attachment
                .after(QaCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

fn handle_editor_load_attachment(
    draft: Option<ResMut<AttachmentDraft>>,
    registry: Option<Res<AttachmentRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorLoadAttachment>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut draft), Some(registry)) = (draft, registry) else {
        for (_args, responder) in take_calls::<EditorLoadAttachment>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, REGISTRY_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorLoadAttachment>(&mut queue) {
        let outcome = match load_attachment(&mut draft, &registry, &args.key) {
            KeyLookup::Loaded => EditorLoadOutcomeNet::Loaded { key: args.key },
            KeyLookup::NoSuchKey(known) => EditorLoadOutcomeNet::NoSuchKey {
                key: args.key,
                known,
            },
        };
        responder.answer(&EditorLoadAttachmentReply { outcome });
    }
}
