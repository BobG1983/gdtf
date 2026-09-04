//! `editor.validation`: how far the content integrity pass has got, and what it found.

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_assets::{ContentChecksComplete, ContentIntegrityReport, ContentValidationDone};
use serde::{Deserialize, Serialize};

use crate::mcp::{
    facts::EditorFacts,
    schedule::EditorMcpSystems,
    wire::{ChecksCompleteNet, ValidationFindingNet, ValidationPublishedNet},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorValidationArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorValidationReply {
    checks_complete: ChecksCompleteNet,
    published:       ValidationPublishedNet,
    findings:        Vec<ValidationFindingNet>,
}

pub(in crate::mcp) struct EditorValidation;

impl McpCommand for EditorValidation {
    type Args = EditorValidationArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorValidationReply;

    const NAME: CommandName = CommandName::from_static("editor.validation");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read the content integrity report: every finding the reference checks raised, plus \
         whether those checks have run and whether the report has been published. The report is \
         inserted at plugin build, so this answers from the moment the process boots. An empty \
         findings list before the pass has published is not a clean bill of health.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &EditorFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_validation
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

// One line per finding, through the same `Display` the editor's own log prints.
fn lines(report: Option<&ContentIntegrityReport>) -> Vec<ValidationFindingNet> {
    report.map_or_else(Vec::new, |report| {
        report
            .findings()
            .iter()
            .map(ValidationFindingNet::from_finding)
            .collect()
    })
}

fn handle_editor_validation(
    report: Option<Res<ContentIntegrityReport>>,
    complete: Option<Res<ContentChecksComplete>>,
    published: Option<Res<ContentValidationDone>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorValidation>>>,
) {
    if queue.is_empty() {
        return;
    }
    let checks_complete = ChecksCompleteNet::new(complete.is_some());
    let published = ValidationPublishedNet::new(published.is_some());
    let findings = lines(report.as_deref());
    for (_args, responder) in take_calls::<EditorValidation>(&mut queue) {
        responder.answer(&EditorValidationReply {
            checks_complete,
            published,
            findings: findings.clone(),
        });
    }
}
