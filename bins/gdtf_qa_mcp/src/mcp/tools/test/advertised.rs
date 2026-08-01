//! What `tools/list` advertises: the tool set a client can see, and the argument schemas
//! and descriptions it reads to use them.

use gdtf_qa_protocol::view::EditorQueryKind;

use crate::mcp::tools::tools_list_result;

/// The two courier tools name NO command — not in their schemas, not in their descriptions.
///
/// The property the whole command layer rests on: a host's vocabulary is read from the
/// RUNNING host and carried as data, so the courier must be able to reach a command it has
/// never heard of. A schema that enumerated command names, or a description that listed
/// them, would be a second copy of that vocabulary — stale the moment either host gains a
/// command, and a client only ever sends what `tools/list` advertises.
///
/// It checks the whole descriptor as text, not just the `command` property, because the
/// leak this guards against is a name written anywhere a client reads.
#[test]
fn neither_courier_tool_names_a_command() {
    let result = tools_list_result();
    let Some(tools) = result["tools"].as_array() else {
        unreachable!("tools/list result carries a `tools` array");
    };
    for wire_name in ["commands", "run"] {
        let Some(tool) = tools
            .iter()
            .find(|tool| tool["name"].as_str() == Some(wire_name))
        else {
            unreachable!("tools/list advertises {wire_name}");
        };
        assert!(
            tool["inputSchema"]["properties"]["command"]["enum"].is_null(),
            "{wire_name} must not enumerate command names: {}",
            tool["inputSchema"],
        );
        let descriptor = tool.to_string();
        // `app.phase` is the only command that exists today, so it is the only name that
        // COULD have leaked; the dotted shape is what every later command will share.
        assert!(
            !descriptor.contains("app.phase"),
            "{wire_name} names a command: {descriptor}"
        );
    }
}

/// `run` advertises the two per-call riders and leaves `arguments` unshaped.
///
/// A client only sends what `tools/list` advertises, so an unadvertised `await_ready` is an
/// unreachable rider (the GTW-875 lesson: the host parsed five launch arguments perfectly
/// and no client ever sent one). `arguments` carries no `properties` on purpose — its shape
/// is the COMMAND's, published by `commands` at `detail: "Full"`.
#[test]
fn run_advertises_its_riders_and_leaves_arguments_unshaped() {
    let result = tools_list_result();
    let Some(tools) = result["tools"].as_array() else {
        unreachable!("tools/list result carries a `tools` array");
    };
    let Some(run) = tools
        .iter()
        .find(|tool| tool["name"].as_str() == Some("run"))
    else {
        unreachable!("tools/list advertises run");
    };
    let properties = &run["inputSchema"]["properties"];
    for argument in ["command", "arguments", "host", "await_ready", "capture"] {
        assert!(
            properties[argument].is_object(),
            "run advertises `{argument}`: {}",
            run["inputSchema"],
        );
    }
    assert_eq!(
        run["inputSchema"]["required"],
        serde_json::json!(["command"])
    );
    assert!(
        properties["arguments"]["properties"].is_null(),
        "`arguments` is the command's shape, not one written into the tool: {}",
        run["inputSchema"],
    );
}

/// `commands` advertises its detail level as a schema `enum` drawn from the parser's own
/// levels, so the schema cannot offer a word the parser rejects.
#[test]
fn commands_advertises_its_detail_levels() {
    let result = tools_list_result();
    let Some(tools) = result["tools"].as_array() else {
        unreachable!("tools/list result carries a `tools` array");
    };
    let Some(commands) = tools
        .iter()
        .find(|tool| tool["name"].as_str() == Some("commands"))
    else {
        unreachable!("tools/list advertises commands");
    };
    assert_eq!(
        commands["inputSchema"]["properties"]["detail"]["enum"],
        serde_json::json!(["Summary", "Full"]),
    );
}

/// `tools/list` advertises exactly the eighteen implemented tools — the fourteen forwarding
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
/// The two COURIER tools are asserted PRESENT for the SAME reason (GTW-942): the game host
/// answers `QaRequest::Catalogue` and `QaRequest::Run`, so without a client tool for each,
/// its whole command layer would be unreachable from any agent — and every command added
/// after it, forever, since the pair is the only route to one.
#[test]
fn lists_every_tool_including_focus_control() {
    let result = tools_list_result();
    let Some(tools) = result["tools"].as_array() else {
        unreachable!("tools/list result carries a `tools` array");
    };
    assert_eq!(tools.len(), 18);
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
        "commands",
        "run",
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
