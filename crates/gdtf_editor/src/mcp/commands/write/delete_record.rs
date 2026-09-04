//! `editor.delete_record` — delete one authored record, refusing while it is in use.

use bevy::prelude::*;
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, DeferredReplies, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    ArgumentFault, CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote,
    UnavailableCode,
};
use gdtf_assets::{ContentMemberKey, FindingFamily};
use serde::Deserialize;

use crate::{
    delete::{
        DeleteOutcome, DeleteRegistry, DeleteRequest, OfferResolution, ReplacementOffer,
        entries_offered_on,
    },
    mcp::{
        commands::availability::only_while_editing,
        facts::EditorFacts,
        schedule::EditorMcpSystems,
        wire::{DeleteCancelNet, DeleteFamilyNet, DeleteKeyNet, DeleteOutcomeNet},
    },
    mode::{EditorMode, InjurySubTab},
};

const NOT_EDITING: RefusalNote = RefusalNote::from_static(
    "editor.delete_record drives the authoring session's own delete, which the editor only \
     builds on entering Editing",
);

const NOT_ON_THIS_SCREEN: RefusalNote = RefusalNote::from_static(
    "a record is deleted from the screen that shows it, so this delete needs its own mode tab \
     and sub-tab open",
);

const BOTH_MEANINGS: &str = "editor.delete_record takes either a replacement key, which confirms \
                             the delete, or cancel, which calls it off. One call carries one \
                             meaning, so naming both is refused.";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorDeleteRecordArgs {
    family:      DeleteFamilyNet,
    key:         DeleteKeyNet,
    /// The record every kept reference is pointed at. Naming one confirms the delete.
    #[serde(default)]
    replacement: Option<DeleteKeyNet>,
    /// Call the delete off, writing nothing and removing nothing.
    #[serde(default)]
    cancel:      DeleteCancelNet,
}

/// The replacement a parked call chose, written into the offer when one opens.
#[derive(Resource)]
pub(in crate::mcp) struct EditorDeleteChoice(Option<ContentMemberKey>);

/// What one parked delete asked for, so its reply matches its own outcome.
pub(in crate::mcp) struct EditorDeleteTicket {
    family: FindingFamily,
    key:    ContentMemberKey,
}

pub(in crate::mcp) struct EditorDeleteRecord;

impl McpCommand for EditorDeleteRecord {
    type Args = EditorDeleteRecordArgs;
    type Facts = EditorFacts;
    type Parked = EditorDeleteTicket;
    type Reply = DeleteOutcomeNet;

    const NAME: CommandName = CommandName::from_static("editor.delete_record");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Delete one authored record, naming its finding family label and its registry key. The \
         in-use check runs first. Naming a replacement key confirms the delete and points every \
         kept reference at that record; omitting one on a record with a required referrer comes \
         back Refused(InUse) with every referring record. `cancel` calls the delete off, writing \
         nothing and removing nothing, and `cancel` together with a replacement key is refused as \
         BadArguments. A family this build cannot delete comes back Refused(NoEntry), and a \
         family the open mode and sub-tab does not offer answers Unavailable { code: WrongState \
         }.",
    );
    const TIMING: CommandTiming = CommandTiming::Deferred;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_while_editing(*facts, NOT_EDITING)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (handle_editor_delete_record, answer_replacement_offer)
                .chain()
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
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
        if *args.cancel && args.replacement.is_some() {
            responder.bad_arguments(ArgumentFault::new(BOTH_MEANINGS.to_owned()));
            continue;
        }
        if registry.handles(&family) && !offers(&registry, open_mode, open_sub_tab, &family) {
            responder.unavailable(UnavailableCode::WrongState, NOT_ON_THIS_SCREEN);
            continue;
        }
        if *args.cancel {
            commands.insert_resource(DeleteOutcome::Cancelled);
        } else {
            let choice = args
                .replacement
                .as_ref()
                .map(|replacement| ContentMemberKey::new((**replacement).clone()));
            commands.insert_resource(EditorDeleteChoice(choice));
            commands.insert_resource(DeleteRequest::new(family.clone(), key.clone()));
        }
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
            "editor mcp: a delete settled",
        );
        Some(settled.clone())
    });
    if *delivered > 0 {
        commands.remove_resource::<DeleteOutcome>();
        commands.remove_resource::<EditorDeleteChoice>();
    }
}

// A call answers the offer the delete opens, so no author has to be at the shell.
fn answer_replacement_offer(
    offer: Option<ResMut<ReplacementOffer>>,
    choice: Option<Res<EditorDeleteChoice>>,
    mut commands: Commands,
) {
    let (Some(mut offer), Some(choice)) = (offer, choice) else {
        return;
    };
    if offer.resolve.is_some() {
        return;
    }
    offer.choose.clone_from(&choice.0);
    offer.resolve = Some(OfferResolution::Confirm);
    commands.remove_resource::<EditorDeleteChoice>();
}
