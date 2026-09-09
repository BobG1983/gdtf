//! This game registers two hosts, each with its own package, port and policy.

use cobalt_mcp_protocol::ports::McpPort;
use cobalt_mcp_server::{HostName, LaunchPolicy, initialize_result};
use mcp::{
    hosts::{DEFAULT_EDITOR_PORT, DEFAULT_GAME_PORT},
    identity, registry,
};

// A registered name as a tool call spells it.
fn named(name: &str) -> HostName {
    HostName::new(name.to_owned())
}

#[test]
fn the_game_and_the_editor_are_both_registered_with_their_own_package_and_port() {
    let registry = registry();

    let names: Vec<&str> = registry.names().iter().map(|name| name.as_str()).collect();
    assert_eq!(names, vec!["game", "editor"], "both hosts are registered");
    let (Some(game), Some(editor)) = (registry.get(&named("game")), registry.get(&named("editor")))
    else {
        unreachable!("both registered names resolve");
    };
    assert_eq!(game.package().as_str(), "game");
    assert_eq!(editor.package().as_str(), "editor");
    let expected_game: McpPort = DEFAULT_GAME_PORT;
    let expected_editor: McpPort = DEFAULT_EDITOR_PORT;
    assert_eq!(
        game.default_port(),
        expected_game,
        "the game host binds the port the game constant names"
    );
    assert_eq!(
        editor.default_port(),
        expected_editor,
        "the editor host binds the port the editor constant names"
    );
    assert_ne!(
        game.default_port(),
        editor.default_port(),
        "two hosts on one port would fight over the listener"
    );
}

#[test]
fn the_editor_starts_another_child_per_launch_while_the_game_keeps_one() {
    let registry = registry();

    let (Some(game), Some(editor)) = (registry.get(&named("game")), registry.get(&named("editor")))
    else {
        unreachable!("both registered names resolve");
    };
    assert_eq!(game.launch_policy(), LaunchPolicy::Reuse);
    assert_eq!(editor.launch_policy(), LaunchPolicy::AlwaysSpawn);
}

#[test]
fn the_advertised_server_name_is_the_one_mcp_json_registers() {
    let advertised = initialize_result(&identity(), None);

    assert_eq!(
        advertised["serverInfo"]["name"],
        serde_json::json!("gdtf-mcp"),
        "`.mcp.json` registers this server under that name"
    );
}
