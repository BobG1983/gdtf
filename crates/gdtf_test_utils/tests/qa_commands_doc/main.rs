//! QA commands guide: cited paths exist and shown shapes match the worked example.
use std::{
    fs,
    path::{Path, PathBuf},
};

const GUIDE: &str = "docs/tooling/qa-commands.md";

const WORKED_EXAMPLE: &str = "crates/gdtf_app/src/dev/net_qa/commands/read/app_phase.rs";

const SHOWN_IN_THE_EXAMPLE: &[&str] = &[
    "AppPhaseArgs",
    "AppPhaseReply",
    "AppPhase",
    "handle_app_phase",
    "deny_unknown_fields",
    "type Facts = GameFacts;",
    "const NAME: CommandName = CommandName::from_static(\"app.phase\")",
    "const TIMING: CommandTiming = CommandTiming::Immediate;",
    "fn availability(",
    "fn register_handler(",
    "QaCommandSystems::Claim",
    "take_calls::<AppPhase>",
    "PendingQueue<CommandCall<AppPhase>>",
];

const CITED_PATHS: &[&str] = &[
    "crates/gdtf_app/src/dev/net_qa/commands/set.rs",
    "crates/gdtf_app/src/dev/net_qa/facts/game_facts.rs",
    "crates/gdtf_app/src/dev/net_qa/wire/phase.rs",
    "crates/gdtf_app/tests/net_qa/commands.rs",
    "crates/gdtf_app/tests/net_qa/command_set.rs",
];

fn repo_root() -> PathBuf {
    std::env::var_os("GDTF_QA_COMMANDS_DOC_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    let Ok(text) = fs::read_to_string(&path) else {
        unreachable!("{relative} must exist at {}", path.display());
    };
    text
}

#[test]
fn the_guide_cites_its_worked_example_by_path() {
    let guide = read(GUIDE);
    assert!(
        guide.contains(WORKED_EXAMPLE),
        "the guide must cite {WORKED_EXAMPLE} by path, so a reader can open the code it \
         describes",
    );
    let mut missing: Vec<&str> = Vec::new();
    for cited in std::iter::once(WORKED_EXAMPLE).chain(CITED_PATHS.iter().copied()) {
        if !repo_root().join(cited).exists() {
            missing.push(cited);
        }
        assert!(
            guide.contains(cited),
            "this guard's `CITED_PATHS` claims the guide points at {cited}, and it does not \
             — fix whichever of the two is wrong",
        );
    }
    assert!(
        missing.is_empty(),
        "the guide points at paths that do not exist: {missing:?}",
    );
}

#[test]
fn every_shape_the_guide_shows_exists_in_the_worked_example() {
    let guide = read(GUIDE);
    let example = read(WORKED_EXAMPLE);
    let mut absent: Vec<&str> = Vec::new();
    let mut unshown: Vec<&str> = Vec::new();
    for shape in SHOWN_IN_THE_EXAMPLE {
        if !example.contains(shape) {
            absent.push(shape);
        }
        if !guide.contains(shape) {
            unshown.push(shape);
        }
    }
    assert!(
        absent.is_empty(),
        "the guide shows shapes that are not in {WORKED_EXAMPLE}: {absent:?}",
    );
    assert!(
        unshown.is_empty(),
        "this guard claims the guide shows these and it does not — fix whichever is wrong: \
         {unshown:?}",
    );
}

#[test]
fn the_guide_names_no_command_that_was_never_built() {
    let guide = read(GUIDE);
    for invented in ["act.fire", "act.move", "battle.state"] {
        assert!(
            !guide.contains(invented),
            "the guide names `{invented}`, which no host offers — the worked example must be \
             real code, cited by path",
        );
    }
}
