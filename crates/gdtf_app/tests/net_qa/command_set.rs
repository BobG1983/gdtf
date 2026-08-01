//! GTW-942 clauses 9–10: facts about the GAME host's command SET and its registration that
//! no request/response case can see.
//!
//! Four of them, each guarding a different way the scaffold could rot: the two per-host
//! conformance assertions run over the REAL slice, the set being exactly the one command this
//! ticket lands, clause 10 — exactly ONE system drains `Res<NetInbox>` in the game — and the
//! placement clause, that the command sets run inside `InputSystems::Gather`.
//!
//! Clause 9 (`crates/gdtf_app/src/states/` is unmodified) is deliberately NOT here: it is a
//! property of a DIFF, not of a running app, and a test that read those files could only
//! restate their current contents. The wire mirrors in `crate::dev::net_qa::wire` are what
//! make it true — they exist precisely so no `schemars` derive has to go inside the
//! `support_item!` invocations over there.

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

/// The `crates/gdtf_app` directory this test binary was compiled from.
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The host's set passes both conformance assertions `gdtf_qa_command` publishes.
///
/// They live in that crate but are FACTS ABOUT THIS SLICE, so they have to be called with it.
/// A duplicate name would make `admit`'s linear scan resolve to whichever entry came first
/// and silently shadow the other; a schema document that is not JSON would ride the wire
/// unnoticed, because a catalogue row carries its schemas as opaque text.
#[test]
fn the_game_command_set_is_conformant() {
    assert_game_command_set_is_conformant();
}

/// The set is exactly `app.phase` — the ONE list, with the ONE entry this ticket lands.
///
/// It is not a count: a wrong command with the right count would pass a count, and the point
/// of the scaffold is that the NEXT ticket adds a file plus one line here.
#[test]
fn the_game_offers_exactly_app_phase() {
    assert_eq!(
        game_command_names(),
        vec![CommandName::from_static("app.phase")],
    );
}

/// **Clause 10.** Exactly ONE system drains `Res<NetInbox>` in the game.
///
/// `NetInbox::drain()` is `rx.try_iter().collect()` — it takes EVERYTHING in the channel — so
/// two systems reading that resource in one frame means whichever runs first swallows the
/// other's requests, nondeterministically. That is why GTW-942 WIDENED `route_requests` with
/// the `Catalogue` and `Run` arms instead of registering a command router of its own.
///
/// The pin is in two halves, because neither alone is enough:
///
/// - the SOURCE half — the `net_qa` module contains exactly one call to `.drain()` on the
///   inbox, so there is one function that could drain;
/// - the SCHEDULE half — the real app, built through the real plugin, holds exactly ONE
///   system in the command layer's `Route` band, which is where the drain is registered.
///
/// Together: one function can drain, and it is registered once. A second router would fail
/// the first half; a second `add_systems` of the existing one — the mistake this ticket
/// nearly made while putting `route_requests` into the `Route` band — fails the second.
///
/// The schedule half counts SET MEMBERSHIP rather than system names because bevy's `debug`
/// feature is off in this build, so every `System::name()` reads
/// "&lt;Enable the debug feature to see the name&gt;" and a name-based count would silently
/// match everything or nothing.
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
                // The call, not the doc comments and module prose that DESCRIBE it — several
                // of which quote `NetInbox::drain()` precisely because this rule matters.
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
    // One frame so the executor initialises: `Schedule::systems()` reports nothing until the
    // schedule has run at least once.
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

/// The command sets sit INSIDE the game's `InputSystems::Gather` band, which is the ticket's
/// placement clause.
///
/// Without this, deleting the whole `configure_sets` call in `commands/register.rs` leaves the
/// suite green: `Route` would still exist as a free-floating set with the router in it, so the
/// count above still passes, and a `Run` would be claimed at an unordered point in `Update`
/// relative to the input the command reads.
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
