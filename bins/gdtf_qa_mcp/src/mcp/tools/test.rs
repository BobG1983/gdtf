//! Registry pins: the advertised tool set and the wire-name round-trip.

use crate::mcp::tools::{ToolName, name::ALL, tools_list_result};

/// `tools/list` advertises exactly the twelve implemented tools — the ten forwarding
/// tools plus the two lifecycle tools.
///
/// `start_battle` is asserted PRESENT: the game has serviced `QaRequest::StartBattle`
/// since T9 (GTW-742), but no client tool sent it, so an agent could never reach a
/// battle over the wire and the battle-only tools stayed unavailable forever. That
/// gap survived a green suite because the only coverage was game-side (GTW-760).
/// `stepper_control` is asserted PRESENT for the SAME reason (GTW-766), and
/// `activate_menu_item` for the SAME reason (GTW-787). `focus_control` is asserted
/// PRESENT for the SAME reason (GTW-802): the game services `QaRequest::FocusControl`,
/// so a missing client tool would leave every off-battle screen un-drivable — the exact
/// gap that ticket was filed for.
#[test]
fn lists_every_tool_including_focus_control() {
    let result = tools_list_result();
    let Some(tools) = result["tools"].as_array() else {
        unreachable!("tools/list result carries a `tools` array");
    };
    assert_eq!(tools.len(), 12);
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    for expected in [
        "send_input",
        "query_state",
        "get_output",
        "take_screenshot",
        "screenshot_after",
        "app_flow",
        "launch_game",
        "stop_game",
        "start_battle",
        "stepper_control",
        "activate_menu_item",
        "focus_control",
    ] {
        assert!(
            names.contains(&expected),
            "tools/list advertises {expected}"
        );
    }
}

/// Every wire name round-trips through `from_wire`, and an unknown name resolves to
/// `None`. Walking `ALL` (rather than a hand-listed table) keeps a newly added tool in.
#[test]
fn wire_names_round_trip() {
    for tool in ALL.iter().copied() {
        assert_eq!(ToolName::from_wire(tool.wire_name()), Some(tool));
    }
    assert_eq!(ToolName::from_wire("nope"), None);
}
