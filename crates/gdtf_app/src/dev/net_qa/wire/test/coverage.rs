//! decoded one, so `#[serde(skip)]` on a live `GamePhaseNet` arm shipped green.
use std::{
    fs,
    path::{Path, PathBuf},
};

const ROUND_TRIP_CALL: &str = "assert_ron_round_trip(";

const SCHEMA_CALL: &str = "assert_schema_is_usable::<";

fn wire_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/dev/net_qa/wire")
}

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

fn names_identifier(haystack: &str, wanted: &str) -> bool {
    let boundary = |character: char| !(character.is_alphanumeric() || character == '_');
    haystack.match_indices(wanted).any(|(at, _)| {
        let before_ok = at == 0 || haystack[..at].chars().next_back().is_none_or(boundary);
        let after = at + wanted.len();
        let after_ok = haystack[after..].chars().next().is_none_or(boundary);
        before_ok && after_ok
    })
}

fn code_only(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

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
