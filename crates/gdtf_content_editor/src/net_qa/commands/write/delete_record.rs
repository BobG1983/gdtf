//! `editor.delete_record` — delete one authored record, refusing while it is in use.

use bevy::prelude::*;
use gdtf_assets::{ContentMemberKey, FindingFamily};
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredReplies, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::Deserialize;

use crate::{
    delete::{DeleteOutcome, DeleteRegistry, DeleteRequest, entries_offered_on},
    mode::{EditorMode, InjurySubTab},
    net_qa::{
        commands::availability::only_while_editing,
        facts::EditorFacts,
        schedule::EditorNetQaSystems,
        wire::{DeleteFamilyNet, DeleteKeyNet, DeleteOutcomeNet},
    },
};

const NOT_EDITING: RefusalNote = RefusalNote::from_static(
    "editor.delete_record drives the authoring session's own delete, which the editor only \
     builds on entering Editing",
);

const NOT_ON_THIS_SCREEN: RefusalNote = RefusalNote::from_static(
    "a record is deleted from the screen that shows it, so this delete needs its own mode tab \
     and sub-tab open",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorDeleteRecordArgs {
    family: DeleteFamilyNet,
    key:    DeleteKeyNet,
}

/// What one parked delete asked for, so its reply matches its own outcome.
pub(in crate::net_qa) struct EditorDeleteTicket {
    family: FindingFamily,
    key:    ContentMemberKey,
}

pub(in crate::net_qa) struct EditorDeleteRecord;

impl QaCommand for EditorDeleteRecord {
    type Args = EditorDeleteRecordArgs;
    type Facts = EditorFacts;
    type Parked = EditorDeleteTicket;
    type Reply = DeleteOutcomeNet;

    const NAME: CommandName = CommandName::from_static("editor.delete_record");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Delete one authored record, naming its finding family label and its registry key. The \
         in-use check runs first, so a record another record still references comes back \
         Refused(InUse) with every referring record. A family this build cannot delete comes \
         back Refused(NoEntry), and a family the open mode and sub-tab does not offer answers \
         Unavailable { code: WrongState }.",
    );
    const TIMING: CommandTiming = CommandTiming::Deferred;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_while_editing(*facts, NOT_EDITING)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_delete_record
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

// The open screen, and whether it offers the delete the call named.
fn offers(
    registry: &DeleteRegistry,
    mode: Option<EditorMode>,
    sub_tab: Option<InjurySubTab>,
    family: &FindingFamily,
) -> bool {
    mode.is_some_and(|mode| {
        entries_offered_on(registry, mode, sub_tab)
            .iter()
            .any(|entry| **entry.family() == **family)
    })
}

fn handle_editor_delete_record(
    registry: Res<DeleteRegistry>,
    mode: Option<Res<EditorMode>>,
    sub_tab: Option<Res<InjurySubTab>>,
    outcome: Option<Res<DeleteOutcome>>,
    mut commands: Commands,
    mut queue: ResMut<PendingQueue<CommandCall<EditorDeleteRecord>>>,
    mut deferred: ResMut<DeferredReplies<EditorDeleteRecord>>,
) {
    let open_mode = mode.map(|mode| *mode);
    let open_sub_tab = sub_tab.map(|sub_tab| *sub_tab);
    for (args, responder) in take_calls::<EditorDeleteRecord>(&mut queue) {
        let family = FindingFamily::new((*args.family).clone());
        let key = ContentMemberKey::new((*args.key).clone());
        if registry.handles(&family) && !offers(&registry, open_mode, open_sub_tab, &family) {
            responder.unavailable(UnavailableCode::WrongState, NOT_ON_THIS_SCREEN);
            continue;
        }
        commands.insert_resource(DeleteRequest::new(family.clone(), key.clone()));
        deferred.park(responder, EditorDeleteTicket { family, key });
    }
    if deferred.is_empty() {
        return;
    }
    let Some(outcome) = outcome else {
        return;
    };
    let settled = DeleteOutcomeNet::from_outcome(&outcome);
    let delivered = deferred.answer_resolved(|ticket| {
        debug!(
            family = %**ticket.family,
            key = %*ticket.key,
            "editor net_qa: a delete settled",
        );
        Some(settled.clone())
    });
    if *delivered > 0 {
        commands.remove_resource::<DeleteOutcome>();
    }
}
