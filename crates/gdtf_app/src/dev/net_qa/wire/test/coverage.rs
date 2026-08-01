//! The pin that keeps the round-trip and schema suites COMPLETE (GTW-944).
//!
//! A hand-written list of cases rots the first time someone adds a type and forgets one, and
//! Rust has no way to enumerate a module's types at compile time. So this reads the `wire/`
//! sources: every published `struct` / `enum` declared there must be named by a case that
//! actually round-trips a value AND by one that actually builds a schema. Add a type without
//! either and this fails, loudly, with the type's name.
//!
//! It scans the DIRECTORY rather than a file list, so a whole new `wire/*.rs` is covered
//! too — that is the case a hand-list misses hardest.
//!
//! # What counts as a case
//!
//! Only the BODY of a test function that performs the assertion. Comments are stripped
//! first, and an import, a helper, or a test that merely names a type are all outside every
//! such body. The first cut of this file took whole FILES instead, and the hole was real:
//! the six `phase` mirrors satisfied it on their `use` line alone while nothing anywhere
//! decoded one, so `#[serde(skip)]` on a live `GamePhaseNet` arm shipped green.
//! [`a_name_outside_a_round_trip_case_is_not_a_case`] pins the tightened reader against that
//! exact shape.

use std::{
    fs,
    path::{Path, PathBuf},
};

/// The call that makes a test function a round-trip case.
const ROUND_TRIP_CALL: &str = "assert_ron_round_trip(";

/// The call that makes a test function a derived-schema case.
const SCHEMA_CALL: &str = "assert_schema_is_usable::<";

/// The `wire/` source directory, resolved from the crate this test is compiled into.
fn wire_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/dev/net_qa/wire")
}

/// Every `*.rs` file directly inside `dir`, as `(file name, contents)`, sorted by name.
///
/// Fails loudly (the house `let Ok(..) else { unreachable!() }` idiom) if the directory or
/// any file cannot be read — a wire module this test cannot see is a test that proves
/// nothing.
fn rust_sources(dir: &Path) -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir(dir) else {
        unreachable!("the wire source directory `{}` is readable", dir.display());
    };
    let mut sources = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            unreachable!("every entry of `{}` is readable", dir.display());
        };
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            unreachable!("`{}` has a readable file name", path.display());
        };
        let Ok(text) = fs::read_to_string(&path) else {
            unreachable!("`{}` is readable", path.display());
        };
        sources.push((name.to_owned(), text));
    }
    sources.sort();
    sources
}

/// The type `line` declares, if it declares a published `struct` or `enum`.
///
/// Matches any visibility a wire type could wear — `pub`, `pub(crate)`, `pub(super)` — so
/// narrowing a type's visibility does not quietly drop it out of the scan.
fn declared_type(line: &str) -> Option<String> {
    let after_visibility = line.trim_start().strip_prefix("pub")?;
    let after_visibility = match after_visibility.split_once(')') {
        Some((scope, rest)) if scope.starts_with('(') => rest,
        _ => after_visibility,
    };
    let rest = after_visibility
        .trim_start()
        .strip_prefix("struct ")
        .or_else(|| after_visibility.trim_start().strip_prefix("enum "))?;
    let name: String = rest
        .chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// Every published `struct` / `enum` declared in the `wire/` sources, as
/// `(file name, type name)`.
fn declared_wire_types() -> Vec<(String, String)> {
    let mut declared = Vec::new();
    for (file, text) in rust_sources(&wire_dir()) {
        for line in text.lines() {
            if let Some(name) = declared_type(line) {
                declared.push((file.clone(), name));
            }
        }
    }
    declared
}

/// Whether `haystack` names `wanted` as a whole identifier — `LevelNet` must not be matched
/// by the `CellLevelNet` that contains it.
fn names_identifier(haystack: &str, wanted: &str) -> bool {
    let boundary = |character: char| !(character.is_alphanumeric() || character == '_');
    haystack.match_indices(wanted).any(|(at, _)| {
        let before_ok = at == 0 || haystack[..at].chars().next_back().is_none_or(boundary);
        let after = at + wanted.len();
        let after_ok = haystack[after..].chars().next().is_none_or(boundary);
        before_ok && after_ok
    })
}

/// `text` without its line comments — the prose a type name can be mentioned in without
/// anyone having written a case for it.
fn code_only(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The bodies of `text`'s top-level functions that actually CALL `assertion`, concatenated.
///
/// This is what makes naming a type a case rather than a mention: an import, a `const`
/// table, a helper and a test that never calls `assertion` all sit outside every collected
/// body. `rustfmt` starts every top-level item at column 0 and closes it with a `}` at
/// column 0, so a function body is the run of lines between the two.
fn bodies_calling(text: &str, assertion: &str) -> String {
    let code = code_only(text);
    let mut bodies = String::new();
    let mut body = String::new();
    let mut inside = false;
    for line in code.lines() {
        if inside {
            if line == "}" {
                if body.contains(assertion) {
                    bodies.push_str(&body);
                }
                body.clear();
                inside = false;
            } else {
                body.push_str(line);
                body.push('\n');
            }
        } else {
            inside = !line.starts_with(char::is_whitespace)
                && line.split_whitespace().any(|word| word == "fn");
        }
    }
    bodies
}

/// The test sources, split into the round-trip corpus and the schema corpus — the bodies of
/// the cases themselves, nothing else.
///
/// The round-trip corpus is every test file that is not wiring (`mod.rs`), the shared
/// assertion (`support.rs`), this file, or the schema suite — so a NEW round-trip file
/// counts without editing anything here.
fn test_corpora() -> (String, String) {
    let mut round_trip = String::new();
    let mut schema = String::new();
    for (file, text) in rust_sources(&wire_dir().join("test")) {
        match file.as_str() {
            "mod.rs" | "support.rs" | "coverage.rs" => {}
            "schema.rs" => schema.push_str(&bodies_calling(&text, SCHEMA_CALL)),
            _ => round_trip.push_str(&bodies_calling(&text, ROUND_TRIP_CALL)),
        }
    }
    (round_trip, schema)
}

/// The scan itself works — it finds a type in EVERY wire source file.
///
/// Without this, a scanner broken by a formatting change would find nothing and the two
/// tests below would pass vacuously. Per-file rather than a total count: a bare total has
/// slack, so one file could drop out of the scan entirely and still clear it.
#[test]
fn the_wire_scan_finds_the_vocabulary() {
    let declared = declared_wire_types();
    for (file, _) in rust_sources(&wire_dir()) {
        assert!(
            file == "mod.rs" || declared.iter().any(|(scanned, _)| *scanned == file),
            "the wire scan found no type in `wire/{file}`, so it is broken: {declared:?}",
        );
    }
    for wanted in ["GangerToken", "CellLevelNet", "NetIntent", "AppPhaseNet"] {
        assert!(
            declared.iter().any(|(_, name)| name == wanted),
            "the wire scan finds `{wanted}`, got {declared:?}",
        );
    }
}

/// A name that is only MENTIONED is not a case — the hole this file shipped with.
///
/// The fixture is the exact shape that fooled the first cut: a type named by an import, by a
/// helper, and by a test that asserts on its `Debug` text without ever encoding it. None of
/// those is a round trip, so none may count. The second name is the control — a real case
/// must still be found, or the reader could pass this test by returning nothing at all.
#[test]
fn a_name_outside_a_round_trip_case_is_not_a_case() {
    let source = "\
use crate::dev::net_qa::wire::phase::MentionedNet;

// MentionedNet in a comment is not a case either.
fn a_helper() -> MentionedNet {
    MentionedNet::Only
}

#[test]
fn a_case_that_never_round_trips() {
    assert_eq!(format!(\"{:?}\", MentionedNet::Only), \"Only\");
}

#[test]
fn a_real_case() {
    assert_ron_round_trip(&CoveredNet::new(1));
}
";
    let corpus = bodies_calling(source, ROUND_TRIP_CALL);
    assert!(
        !names_identifier(&corpus, "MentionedNet"),
        "an import, a helper and a non-round-trip test are not round-trip cases, got \
         `{corpus}`",
    );
    assert!(
        names_identifier(&corpus, "CoveredNet"),
        "a name inside a round-trip case IS a case, got `{corpus}`",
    );
}

/// Every type declared in `wire/` is named by a round-trip test.
#[test]
fn every_wire_type_has_a_round_trip_case() {
    let (round_trip, _) = test_corpora();
    for (file, name) in declared_wire_types() {
        assert!(
            names_identifier(&round_trip, &name),
            "`{name}` (wire/{file}) has no round-trip case — add one beside its siblings in \
             wire/test/",
        );
    }
}

/// Every type declared in `wire/` is named by the schema test.
#[test]
fn every_wire_type_has_a_schema_case() {
    let (_, schema) = test_corpora();
    for (file, name) in declared_wire_types() {
        assert!(
            names_identifier(&schema, &name),
            "`{name}` (wire/{file}) has no derived-schema case — add one to \
             wire/test/schema.rs",
        );
    }
}
