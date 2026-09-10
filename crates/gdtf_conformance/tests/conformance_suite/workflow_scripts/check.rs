//! Fail when a workflow script does not parse, or when the build agent's output schema
//! outgrows the size a build survives.

use std::{fs, path::Path};

use tempfile::TempDir;

use crate::workflow_scripts::{
    parse::{NODE, Outcome, compile},
    schema::measure_build_ticket,
    tree::{WORKFLOW_DIR, repo_root, workflow_scripts},
};

// A fixture script the checker must accept, its template-literal backtick escaped.
const ACCEPTED: &str = r"export const meta = { name: 'fixture' };
const note = `a \` b`;
await Promise.resolve(note);
return note;
";

#[test]
fn every_workflow_script_compiles_the_way_the_harness_loads_it() {
    let root = repo_root();
    let complaints = fixture_complaints(&root);
    assert!(complaints.is_empty(), "{}", complaints.join("\n"));

    let scripts = workflow_scripts(&root);
    assert!(
        !scripts.is_empty(),
        "no `.js` file under {WORKFLOW_DIR} in {}. This guard is reading the wrong \
         directory and cannot see any workflow script",
        root.display(),
    );
    let outcome = compile(&root, &scripts);
    assert!(outcome.is_clean(), "{}", outcome.report());
}

#[test]
fn the_build_agents_output_schema_stays_under_the_size_a_build_survives() {
    let measured = measure_build_ticket(&repo_root());
    assert!(measured.is_within_limit(), "{}", measured.report());
}

// The accepted fixture with its template-literal backtick left bare.
fn rejected_fixture() -> String {
    ACCEPTED.replace("\\`", "`")
}

// One line per way the checker mishandled the two fixtures. Empty when it handled both.
fn fixture_complaints(root: &Path) -> Vec<String> {
    let Ok(dir) = TempDir::new() else {
        return vec![format!(
            "could not write the fixtures that prove the `{NODE}` checker discriminates"
        )];
    };
    let accepted = dir.path().join("accepted.js");
    let rejected = dir.path().join("rejected.js");
    if fs::write(&accepted, ACCEPTED).is_err() || fs::write(&rejected, rejected_fixture()).is_err()
    {
        return vec![format!(
            "could not write the checker fixtures under {}",
            dir.path().display()
        )];
    }
    let accepted = accepted.to_string_lossy().into_owned();
    let rejected = rejected.to_string_lossy().into_owned();

    let mut complaints = Vec::new();
    match compile(root, &[accepted]) {
        Outcome::AllParse => {}
        other => complaints.push(format!(
            "the checker must accept a script whose backtick is escaped. It said: {}",
            other.report()
        )),
    }
    match compile(root, std::slice::from_ref(&rejected)) {
        Outcome::Rejected(found)
            if found.len() == 1 && found.iter().any(|line| line.contains(&rejected)) => {}
        other => complaints.push(format!(
            "the checker must reject the one script whose backtick is bare, naming \
             {rejected}. It said: {}",
            other.report()
        )),
    }
    complaints
}
