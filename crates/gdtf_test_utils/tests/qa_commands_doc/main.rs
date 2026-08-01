//! GTW-942 clause 12 — the how-to-add-a-command guide must describe code that EXISTS.
//!
//! `docs/tooling/qa-commands.md` is the gold standard every future QA command is written
//! from, so a shape it shows that no file contains is worse than no guide at all: an agent
//! building from it produces code that does not compile, or worse, code shaped like a design
//! nobody implemented. The epic has a history of exactly that — a walkthrough written around
//! an `act.fire` that was never built.
//!
//! The guard is deliberately narrow, because a broad one would be a second copy of the file.
//! It checks three things:
//!
//! 1. the guide CITES the worked example by path, and that path exists;
//! 2. every Rust identifier the guide shows in a fenced block, and attributes to that file,
//!    appears in it;
//! 3. the guide does not name a command that does not exist.
//!
//! Std-only, zero cargo invocations — the same shape as the sibling `docs_path_truth`,
//! `module_layout` and `ci_workflow_features` suites. The repo root defaults to
//! `CARGO_MANIFEST_DIR/../..` and can be overridden via `GDTF_QA_COMMANDS_DOC_ROOT`.

use std::{
    fs,
    path::{Path, PathBuf},
};

/// The guide under test.
const GUIDE: &str = "docs/tooling/qa-commands.md";

/// The worked example the guide is anchored to — the file it must cite, and the file every
/// shape it shows must be in.
const WORKED_EXAMPLE: &str = "crates/gdtf_app/src/dev/net_qa/commands/read/app_phase.rs";

/// Every identifier the guide shows and attributes to [`WORKED_EXAMPLE`].
///
/// A hand-kept list on purpose: it is the CLAIM the guide makes, and it is short enough to
/// read. Deriving it from the fenced blocks would only re-extract what the guide already
/// says, so a guide that showed the wrong shape would pass against itself.
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

/// The other repo paths the guide points a reader at, each of which must exist.
///
/// Repo-relative forms only. The guide also carries `../`-relative markdown links (to
/// ADR 0008, and to these same files as clickable links); those are the `docs_path_truth`
/// suite's job, which resolves every inline link target in `docs/` against the live tree.
const CITED_PATHS: &[&str] = &[
    "crates/gdtf_app/src/dev/net_qa/commands/set.rs",
    "crates/gdtf_app/src/dev/net_qa/facts/game_facts.rs",
    "crates/gdtf_app/src/dev/net_qa/wire/phase.rs",
    "crates/gdtf_app/tests/net_qa/commands.rs",
    "crates/gdtf_app/tests/net_qa/command_set.rs",
];

/// The repo root — `GDTF_QA_COMMANDS_DOC_ROOT` override, else `CARGO_MANIFEST_DIR/../..`.
fn repo_root() -> PathBuf {
    std::env::var_os("GDTF_QA_COMMANDS_DOC_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

/// Read a repo-relative file, failing the test with the path if it is not there.
fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    let Ok(text) = fs::read_to_string(&path) else {
        unreachable!("{relative} must exist at {}", path.display());
    };
    text
}

/// The guide cites the worked example BY PATH, and every path it points at exists.
///
/// Citing by path is the clause's own wording, and it is what makes the rest of this suite
/// possible: a guide that only described a shape in prose could not be checked against
/// anything.
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

/// Every shape the guide shows in a fenced block exists in the file it attributes it to.
///
/// This is the clause's load-bearing half: "it must not describe a shape that does not exist
/// in the tree". Both directions are checked — the guide must show it AND the file must
/// contain it — so neither a guide that invented a shape nor a guide that fell behind a
/// rename can pass.
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

/// The guide names no command that does not exist.
///
/// `act.fire` is named explicitly because it is the one that went into this epic's planning
/// material as a walkthrough for code nobody had written, which is what clause 12 was
/// written against.
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
