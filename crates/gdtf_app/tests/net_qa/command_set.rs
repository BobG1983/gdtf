use std::{fs, path::PathBuf};

use bevy::{
    app::Update,
    ecs::{
        intern::Interned,
        schedule::{Schedules, SystemSet},
    },
};
use gdtf_app::test_support::{assert_game_command_set_is_conformant, game_command_names};
use gdtf_battle_input::InputSystems;
use gdtf_qa_command::dispatch::QaCommandSystems;
use gdtf_qa_protocol::command::CommandName;

use super::socket_support::{TestResult, game_app_listening};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn the_game_command_set_is_conformant() {
    assert_game_command_set_is_conformant();
}

#[test]
fn the_game_offers_the_reads_the_lifecycle_the_acts_raw_input_and_the_view_and_battle_controls() {
    assert_eq!(
        game_command_names(),
        vec![
            CommandName::from_static("app.phase"),
            CommandName::from_static("capture.screenshot"),
            CommandName::from_static("settings.read"),
            CommandName::from_static("ui.focus"),
            CommandName::from_static("playback.state"),
            CommandName::from_static("battle.roster"),
            CommandName::from_static("battle.turn"),
            CommandName::from_static("battle.selection"),
            CommandName::from_static("battle.offers"),
            CommandName::from_static("battle.inspect"),
            CommandName::from_static("battle.sightline"),
            CommandName::from_static("battle.visible"),
            CommandName::from_static("battle.cost"),
            CommandName::from_static("log.read"),
            CommandName::from_static("battle.start"),
            CommandName::from_static("battle.flee"),
            CommandName::from_static("procgen.step"),
            CommandName::from_static("wait"),
            CommandName::from_static("act.select"),
            CommandName::from_static("act.select_next"),
            CommandName::from_static("act.select_prev"),
            CommandName::from_static("act.select_clear"),
            CommandName::from_static("act.move"),
            CommandName::from_static("act.fire"),
            CommandName::from_static("act.reload"),
            CommandName::from_static("act.set_stance"),
            CommandName::from_static("act.set_aiming"),
            CommandName::from_static("act.set_facing"),
            CommandName::from_static("act.end_turn"),
            CommandName::from_static("act.melee"),
            CommandName::from_static("act.shove"),
            CommandName::from_static("act.stabilize"),
            CommandName::from_static("act.execute"),
            CommandName::from_static("act.throw_grenade"),
            CommandName::from_static("act.open_door"),
            CommandName::from_static("act.enter_emplacement"),
            CommandName::from_static("act.exit_emplacement"),
            CommandName::from_static("input.press_key"),
            CommandName::from_static("input.hover"),
            CommandName::from_static("input.set_focus"),
            CommandName::from_static("input.focus_step"),
            CommandName::from_static("input.activate"),
            CommandName::from_static("input.click_cell"),
            CommandName::from_static("view.level_up"),
            CommandName::from_static("view.level_down"),
            CommandName::from_static("view.toggle_full_view"),
            CommandName::from_static("view.pan"),
            CommandName::from_static("view.look_at"),
            CommandName::from_static("battle.set_fire_mode"),
        ],
        "the published list is part of the wire contract and does not change with the build's \
         features — only `procgen.step`'s availability code does",
    );
}

#[test]
fn exactly_one_system_drains_the_net_inbox() -> TestResult {
    let net_qa = crate_root().join("src/dev/net_qa");
    let mut drain_sites: Vec<String> = Vec::new();
    let mut pending = vec![net_qa];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir)?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            let text = fs::read_to_string(&path)?;
            for (number, line) in text.lines().enumerate() {
                if line.contains("inbox.drain()") && !line.trim_start().starts_with("//") {
                    drain_sites.push(format!("{}:{}", path.display(), number + 1));
                }
            }
        }
    }
    assert_eq!(
        drain_sites.len(),
        1,
        "exactly one place in the game may drain the inbox, found: {drain_sites:?}",
    );

    let (mut app, _port) = game_app_listening()?;
    app.update();
    let Some(schedules) = app.world().get_resource::<Schedules>() else {
        unreachable!("a built app carries its schedules");
    };
    let Some(update) = schedules.get(Update) else {
        unreachable!("the app carries an Update schedule");
    };
    let route: Interned<dyn SystemSet> = QaCommandSystems::Route.intern();
    let Ok(in_route) = update.graph().systems_in_set(route) else {
        unreachable!(
            "the real plugin puts the router in QaCommandSystems::Route, so the set exists"
        );
    };
    assert_eq!(
        in_route.len(),
        1,
        "the one drain must be registered exactly once — a second `add_systems` of it would \
         manufacture the double drain by hand",
    );
    Ok(())
}

#[test]
fn the_command_sets_run_inside_the_input_gather_band() -> TestResult {
    let (mut app, _port) = game_app_listening()?;
    app.update();
    let Some(schedules) = app.world().get_resource::<Schedules>() else {
        unreachable!("a built app carries its schedules");
    };
    let Some(update) = schedules.get(Update) else {
        unreachable!("the app carries an Update schedule");
    };
    let graph = update.graph();
    let gather: Interned<dyn SystemSet> = InputSystems::Gather.intern();
    let Ok(in_gather) = graph.systems_in_set(gather) else {
        unreachable!("the game registers InputSystems::Gather, so the set exists");
    };
    for set in [QaCommandSystems::Route, QaCommandSystems::Claim] {
        let Ok(members) = graph.systems_in_set(set.intern()) else {
            unreachable!("the real plugin registers {set:?}, so the set exists");
        };
        assert!(
            !members.is_empty(),
            "{set:?} must hold at least one system, or its placement means nothing",
        );
        for member in members {
            assert!(
                in_gather.contains(member),
                "every system in {set:?} must run inside InputSystems::Gather — the band the \
                 ticket places the command layer in",
            );
        }
    }
    Ok(())
}
