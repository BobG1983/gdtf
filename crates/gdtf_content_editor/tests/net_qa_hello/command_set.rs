use std::{fs, path::PathBuf};

use bevy::{
    app::Update,
    ecs::{
        intern::Interned,
        schedule::{Schedules, SystemSet},
    },
};
use gdtf_content_editor::{
    EditorNetQaSystems, assert_editor_command_set_is_conformant, editor_command_names,
};
use gdtf_qa_command::dispatch::QaCommandSystems;
use gdtf_qa_protocol::command::CommandName;

use crate::{client::EDITOR_COMMAND_NAMES, harness::editor_app_listening, support::TestResult};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn the_editor_command_set_is_conformant() {
    assert_editor_command_set_is_conformant();
}

#[test]
fn the_editor_offers_the_lifecycle_commands() {
    let published = editor_command_names();
    for name in EDITOR_COMMAND_NAMES {
        assert!(
            published.contains(&CommandName::from_static(name)),
            "the editor's published list is part of the wire contract, and `{name}` is missing \
             from it: {published:?}",
        );
    }
}

#[test]
fn the_editor_offers_the_reads_the_lifecycle_the_form_writes_the_theme_helpers_and_the_prefab_canvas()
 {
    assert_eq!(
        editor_command_names(),
        vec![
            CommandName::from_static("editor.phase"),
            CommandName::from_static("editor.last_save"),
            CommandName::from_static("editor.validation"),
            CommandName::from_static("editor.families"),
            CommandName::from_static("editor.session"),
            CommandName::from_static("editor.draft"),
            CommandName::from_static("editor.set_mode"),
            CommandName::from_static("editor.new"),
            CommandName::from_static("editor.load"),
            CommandName::from_static("editor.save"),
            CommandName::from_static("editor.set_field"),
            CommandName::from_static("editor.list_op"),
            CommandName::from_static("editor.select_theme"),
            CommandName::from_static("editor.toggle_terrain"),
            CommandName::from_static("editor.set_default_floor"),
            CommandName::from_static("editor.map"),
            CommandName::from_static("editor.set_grid_size"),
            CommandName::from_static("editor.select_tile"),
            CommandName::from_static("editor.set_level"),
            CommandName::from_static("editor.paint"),
            CommandName::from_static("editor.select_injury_tab"),
            CommandName::from_static("editor.select_weighting_table"),
            CommandName::from_static("editor.weighting"),
            CommandName::from_static("editor.save_weighting"),
        ],
        "the published list is part of the wire contract: dropping a name, adding one, or \
         publishing them in another order all change what a client reads back",
    );
}

#[test]
fn exactly_one_system_drains_the_net_inbox() -> TestResult {
    let net_qa = crate_root().join("src/net_qa");
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
        "exactly one place in the editor may drain the inbox, found: {drain_sites:?}",
    );

    let (mut app, _port) = editor_app_listening()?;
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
fn the_command_sets_run_inside_the_editor_gather_set() -> TestResult {
    let (mut app, _port) = editor_app_listening()?;
    app.update();
    let Some(schedules) = app.world().get_resource::<Schedules>() else {
        unreachable!("a built app carries its schedules");
    };
    let Some(update) = schedules.get(Update) else {
        unreachable!("the app carries an Update schedule");
    };
    let graph = update.graph();
    let gather: Interned<dyn SystemSet> = EditorNetQaSystems::Gather.intern();
    let Ok(in_gather) = graph.systems_in_set(gather) else {
        unreachable!("the editor plugin configures EditorNetQaSystems::Gather, so the set exists");
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
                "every system in {set:?} must run inside EditorNetQaSystems::Gather — the set \
                 that carries the editor state machine's run condition",
            );
        }
    }
    let Ok(systems) = update.systems() else {
        unreachable!("a schedule that has run once is initialized");
    };
    let handler: Vec<_> = systems
        .filter(|(_, system)| system.name().to_string().contains("handle_editor_phase"))
        .map(|(key, _)| key)
        .collect();
    let [handler] = handler.as_slice() else {
        unreachable!("`editor.phase` registers its handler exactly once, found {handler:?}");
    };
    assert!(
        in_gather.contains(handler),
        "the command's own handler reads the editor state machine too, so it belongs under the \
         same run condition the set carries",
    );
    Ok(())
}
