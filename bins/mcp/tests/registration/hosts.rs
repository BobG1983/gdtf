//! This game registers two hosts, each with its own package, channel, port and policy.

use cobalt_mcp_server::{HostName, LaunchPolicy, initialize_result};
use mcp::{identity, registry};

// A registered name as a tool call spells it.
fn named(name: &str) -> HostName {
    HostName::new(name.to_owned())
}

#[test]
fn the_game_and_the_editor_are_both_registered_with_their_own_package_and_channel() {
    let registry = registry();

    let names: Vec<&str> = registry.names().iter().map(|name| name.as_str()).collect();
    assert_eq!(names, vec!["game", "editor"], "both hosts are registered");
    let (Some(game), Some(editor)) = (registry.get(&named("game")), registry.get(&named("editor")))
    else {
        unreachable!("both registered names resolve");
    };
    assert_eq!(game.package().as_str(), "game");
    assert_eq!(editor.package().as_str(), "editor");
    assert_eq!(game.channel().enable().as_str(), "GDTF_MCP");
    assert_eq!(editor.channel().enable().as_str(), "GDTF_EDITOR_MCP");
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
