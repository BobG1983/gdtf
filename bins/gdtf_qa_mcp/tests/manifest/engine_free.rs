//! The bridge declares nothing but the shared wire vocabulary, in any section.

// Everything this stdio-to-TCP bridge is allowed to name as a dependency.
const ALLOWED: &[&str] = &["gdtf_qa_protocol", "serde", "serde_json"];

// Section names Cargo reads dependencies from.
const DEPENDENCY_KINDS: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];

fn manifest() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let Ok(text) = std::fs::read_to_string(path) else {
        unreachable!("the crate's own Cargo.toml is readable at {path}");
    };
    text
}

// Split a section header into the part before its final dot and that final segment.
fn split_header(header: &str) -> (Option<&str>, &str) {
    match header.rsplit_once('.') {
        Some((prefix, last)) => (Some(prefix), last),
        None => (None, header),
    }
}

fn is_dependency_kind(segment: &str) -> bool {
    DEPENDENCY_KINDS.contains(&segment)
}

// Every dependency the manifest declares, paired with the section it sits under.
// Covers plain, target-scoped and `[dependencies.<name>]` sub-table forms.
fn declared_dependencies(manifest: &str) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = Vec::new();
    let mut section: Option<String> = None;
    let mut in_sub_table = false;

    for line in manifest.lines() {
        let trimmed = line.trim();
        if let Some(header) = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            let (prefix, last) = split_header(header);
            if is_dependency_kind(last) {
                section = Some(header.to_owned());
                in_sub_table = false;
            } else if let Some(parent) = prefix.filter(|p| is_dependency_kind(split_header(p).1)) {
                found.push((parent.to_owned(), last.to_owned()));
                section = Some(parent.to_owned());
                in_sub_table = true;
            } else {
                section = None;
                in_sub_table = false;
            }
            continue;
        }
        if in_sub_table || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some(current) = section.as_ref() else {
            continue;
        };
        if let Some((name, _)) = trimmed.split_once('=') {
            found.push((current.clone(), name.trim().to_owned()));
        }
    }
    found
}

#[test]
fn no_section_of_the_manifest_reaches_past_the_wire_vocabulary() {
    let found = declared_dependencies(&manifest());

    assert!(
        found.iter().any(|(_, name)| name == "gdtf_qa_protocol"),
        "the parser found no protocol dependency at all, so it is reading nothing and would \
         wave anything through (got {found:?})"
    );
    for (section, name) in &found {
        assert!(
            ALLOWED.contains(&name.as_str()),
            "`{name}` is declared under [{section}] — the bridge speaks the wire protocol and \
             nothing else, so no section may pull in the engine or its transport \
             (allowed: {ALLOWED:?})"
        );
    }
}

#[test]
fn the_parser_reports_entries_from_every_dependency_form() {
    const FIXTURE: &str = r#"
[package]
name = "fixture"

[dependencies]
gdtf_qa_protocol = { path = "x" }

[dev-dependencies]
gdtf_net_qa_transport = { path = "y" }

[target.'cfg(unix)'.build-dependencies]
scoped_crate = "1"

[dependencies.sub_table_crate]
path = "z"
"#;
    let found = declared_dependencies(FIXTURE);

    let expected = [
        ("dev-dependencies", "gdtf_net_qa_transport"),
        ("target.'cfg(unix)'.build-dependencies", "scoped_crate"),
        ("dependencies", "sub_table_crate"),
    ];
    for (section, name) in expected {
        assert!(
            found
                .iter()
                .any(|(got_section, got_name)| got_section == section && got_name == name),
            "the parser must report `{name}` against [{section}] — a form it misses is a form \
             the engine can hide in (got {found:?})"
        );
    }
    assert!(
        !found.iter().any(|(_, name)| name == "path"),
        "keys inside a sub-table are settings, not dependency names (got {found:?})"
    );
}
