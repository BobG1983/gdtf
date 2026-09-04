//! `editor.select_injury_tab`. Opens one of the Injury tab's two sub-tabs.

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use serde::{Deserialize, Serialize};

use crate::{
    InjurySubTab,
    mcp::{
        commands::availability::only_on_the_injury_tab, facts::EditorFacts,
        schedule::EditorMcpSystems, wire::InjurySubTabNet,
    },
};

const NO_SUB_TAB_BAR: RefusalNote = RefusalNote::from_static(
    "editor.select_injury_tab writes the Injury sub-tab resource, which the editor only creates \
     on entering Editing",
);

const NOT_THE_INJURY_TAB: RefusalNote = RefusalNote::from_static(
    "the sub-tab row belongs to the Injury form, so this write needs the Injury tab open",
);

const NO_SUB_TAB_RESOURCE: RefusalNote = RefusalNote::from_static(
    "the Injury sub-tab resource left the world between the availability check and the handler",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorSelectInjuryTabArgs {
    tab: InjurySubTabNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorSelectInjuryTabReply {
    tab: InjurySubTabNet,
}

pub(in crate::mcp) struct EditorSelectInjuryTab;

impl McpCommand for EditorSelectInjuryTab {
    type Args = EditorSelectInjuryTabArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorSelectInjuryTabReply;

    const NAME: CommandName = CommandName::from_static("editor.select_injury_tab");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Open the Injury def form or the Injury weighting table, writing the same `InjurySubTab` \
         resource the sub-tab row writes. Needs the Injury tab open, so it refuses while another \
         top-level tab is showing.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_injury_tab(*facts, NO_SUB_TAB_BAR, NOT_THE_INJURY_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_select_injury_tab
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

fn handle_editor_select_injury_tab(
    sub_tab: Option<ResMut<InjurySubTab>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSelectInjuryTab>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(mut sub_tab) = sub_tab else {
        for (_args, responder) in take_calls::<EditorSelectInjuryTab>(&mut queue) {
            responder.unavailable(UnavailableCode::WrongState, NO_SUB_TAB_RESOURCE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorSelectInjuryTab>(&mut queue) {
        *sub_tab = args.tab.to_sub_tab();
        responder.answer(&EditorSelectInjuryTabReply {
            tab: InjurySubTabNet::from_sub_tab(*sub_tab),
        });
    }
}
