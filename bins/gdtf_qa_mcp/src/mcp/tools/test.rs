//! Registry pins: the advertised tool set and the wire-name round-trip.

use gdtf_qa_protocol::view::EditorQueryKind;

use crate::mcp::tools::{ToolName, name::ALL, tools_list_result};

/// `tools/list` advertises exactly the sixteen implemented tools — the twelve forwarding
/// tools plus the four lifecycle tools.
///
/// `start_battle` is asserted PRESENT: the game has serviced `QaRequest::StartBattle`
/// since T9 (GTW-742), but no client tool sent it, so an agent could never reach a
/// battle over the wire and the battle-only tools stayed unavailable forever. That
/// gap survived a green suite because the only coverage was game-side (GTW-760).
/// `stepper_control` is asserted PRESENT for the SAME reason (GTW-766), and
/// `activate_menu_item` for the SAME reason (GTW-787). `focus_control` is asserted
/// PRESENT for the SAME reason (GTW-802): the game services `QaRequest::FocusControl`,
/// so a missing client tool would leave every off-battle screen un-drivable — the exact
/// gap that ticket was filed for. The four EDITOR tools are asserted PRESENT for the SAME
/// reason (GTW-808): the editor has answered `GetEditorQueryOptions` / `QueryEditor` since
/// GTW-805 and no client tool sent either, so a running editor was unreachable from any
/// agent — the gap the whole GTW-786 epic exists to close.
#[test]
fn lists_every_tool_including_focus_control() {
    let result = tools_list_result();
    let Some(tools) = result["tools"].as_array() else {
        unreachable!("tools/list result carries a `tools` array");
    };
    assert_eq!(tools.len(), 16);
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
        "get_editor_query_options",
        "query_editor",
        "launch_editor",
        "stop_editor",
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

/// `launch_editor` ADVERTISES the same five recipe arguments `launch_game` does, and
/// `query_editor` advertises its topic list as a schema `enum` — a client only sends what
/// `tools/list` advertises, so an unadvertised topic vocabulary is an unusable tool
/// (GTW-808).
#[test]
fn the_editor_tools_advertise_their_arguments() {
    let result = tools_list_result();
    let Some(tools) = result["tools"].as_array() else {
        unreachable!("tools/list result carries a `tools` array");
    };
    let Some(launch) = tools
        .iter()
        .find(|tool| tool["name"].as_str() == Some("launch_editor"))
    else {
        unreachable!("tools/list advertises launch_editor");
    };
    for argument in ["port", "package", "features", "working_dir", "env"] {
        assert!(
            launch["inputSchema"]["properties"][argument].is_object(),
            "launch_editor advertises `{argument}`: {}",
            launch["inputSchema"]
        );
    }
    let Some(description) = launch["description"].as_str() else {
        unreachable!("launch_editor carries a description");
    };
    assert!(description.contains("7617"), "{description}");
    assert!(
        description.contains("gdtf_content_editor_bin"),
        "{description}"
    );

    let Some(query) = tools
        .iter()
        .find(|tool| tool["name"].as_str() == Some("query_editor"))
    else {
        unreachable!("tools/list advertises query_editor");
    };
    let Some(topics) = query["inputSchema"]["properties"]["topic"]["enum"].as_array() else {
        unreachable!(
            "query_editor advertises its topic enum: {}",
            query["inputSchema"]
        );
    };
    let names: Vec<&str> = topics.iter().filter_map(|value| value.as_str()).collect();
    assert_eq!(names.len(), EditorQueryKind::ALL.len());
    for expected in ["Readiness", "Mode", "Session", "Draft", "Validation"] {
        assert!(names.contains(&expected), "query_editor offers {expected}");
    }
    assert_eq!(
        query["inputSchema"]["required"],
        serde_json::json!(["topic"])
    );
}

/// `get_editor_query_options` states its Load-phase behaviour plainly and still tells the
/// caller to read `topics` rather than assume a fixed set.
///
/// The description used to hedge the claim as unconfirmed and point at a follow-up ticket
/// (GTW-882). That hedge is gone: `crates/gdtf_content_editor/tests/net_qa_editor_query/`
/// drives the editor's real plugins over a real listener and connects before the first
/// frame, so it observes the `Load` phase deterministically — better evidence than polling a
/// launched editor, which is a race the asset pass always wins. This test now guards the
/// opposite failure: the stale "never yet by a launched editor process" hedge must not come
/// back (GTW-902).
#[test]
fn the_editor_query_options_description_states_its_load_phase_behaviour() {
    let result = tools_list_result();
    let Some(tools) = result["tools"].as_array() else {
        unreachable!("tools/list result carries a `tools` array");
    };
    let Some(options) = tools
        .iter()
        .find(|tool| tool["name"].as_str() == Some("get_editor_query_options"))
    else {
        unreachable!("tools/list advertises get_editor_query_options");
    };
    let Some(description) = options["description"].as_str() else {
        unreachable!("get_editor_query_options carries a description");
    };
    assert!(description.contains("are absent"), "{description}");
    assert!(
        description.contains("what `topics` actually lists"),
        "{description}"
    );
    assert!(
        !description.contains("never yet by a launched editor process"),
        "the stale unconfirmed-claim hedge is back: {description}"
    );
    assert!(
        !description.contains("in-process tests"),
        "the description should state the behaviour, not cite what backs it: {description}"
    );
}

/// Every tool names a host, and the four editor tools name the EDITOR — a tool that
/// resolved to the game's link would carry an editor request to a process that answers it
/// `BadRequest` (GTW-808).
#[test]
fn every_tool_names_its_host() {
    use crate::hosts::QaHost;

    for tool in ALL.iter().copied() {
        let expected = if matches!(
            tool,
            ToolName::GetEditorQueryOptions
                | ToolName::QueryEditor
                | ToolName::LaunchEditor
                | ToolName::StopEditor
        ) {
            QaHost::Editor
        } else {
            QaHost::Game
        };
        assert_eq!(tool.host(), expected, "{} names its host", tool.wire_name());
    }
    assert!(ToolName::LaunchEditor.is_launch());
    assert!(ToolName::StopEditor.is_stop());
    assert!(!ToolName::QueryEditor.is_launch());
    assert!(!ToolName::QueryEditor.is_stop());
}
