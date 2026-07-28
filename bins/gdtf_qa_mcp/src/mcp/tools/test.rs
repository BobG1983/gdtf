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

/// `launch_game` ADVERTISES its four recipe arguments, and its description names
/// `dev_tools` and `working_dir`.
///
/// The same class of gap the four tickets above were filed for, one level down: the host
/// can parse `package` / `features` / `working_dir` / `env` perfectly and no client will
/// ever send one, because a client only sends what `tools/list` advertises. That is the
/// state the resident host was in when GTW-864 gave up on the MCP harness and hand-wrote a
/// wire client — the server side worked, the advertised argument surface did not carry it
/// (GTW-875).
#[test]
fn launch_game_advertises_every_recipe_argument() {
    let result = tools_list_result();
    let Some(tools) = result["tools"].as_array() else {
        unreachable!("tools/list result carries a `tools` array");
    };
    let Some(launch) = tools
        .iter()
        .find(|tool| tool["name"].as_str() == Some("launch_game"))
    else {
        unreachable!("tools/list advertises launch_game");
    };
    let properties = &launch["inputSchema"]["properties"];
    for argument in ["port", "package", "features", "working_dir", "env"] {
        assert!(
            properties[argument].is_object(),
            "launch_game advertises `{argument}`: {}",
            launch["inputSchema"]
        );
    }
    assert_eq!(properties["package"]["type"], "string");
    assert_eq!(properties["working_dir"]["type"], "string");
    assert_eq!(properties["env"]["type"], "object");
    let Some(description) = launch["description"].as_str() else {
        unreachable!("launch_game carries a description");
    };
    assert!(description.contains("dev_tools"), "{description}");
    assert!(description.contains("working_dir"), "{description}");
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
