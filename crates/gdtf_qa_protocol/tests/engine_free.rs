//! Integration checks that this crate stays free of the Bevy engine and Tokio.

fn manifest() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let Ok(text) = std::fs::read_to_string(path) else {
        unreachable!("the crate's own Cargo.toml is readable at {path}");
    };
    text
}

fn runtime_dependency_names(manifest: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]";
            continue;
        }
        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once('=') {
            names.push(name.trim().to_owned());
        }
    }
    names
}

#[test]
fn crate_depends_on_no_bevy_engine_or_tokio() {
    let names = runtime_dependency_names(&manifest());

    assert!(
        names.iter().any(|name| name == "bevy_derive"),
        "the Deref derive crate must be a direct dep (got {names:?})"
    );
    assert!(
        !names.iter().any(|name| name == "bevy"),
        "the Bevy ENGINE must never be a dep — only bevy_derive (got {names:?})"
    );
    assert!(
        !names.iter().any(|name| name == "tokio"),
        "no async runtime is permitted (got {names:?})"
    );
    for name in &names {
        assert!(
            matches!(name.as_str(), "serde" | "ron" | "bevy_derive" | "schemars"),
            "unexpected runtime dependency `{name}` (allowed: serde, ron, bevy_derive, schemars)"
        );
    }
}

#[test]
fn schemars_is_a_normal_dependency() {
    let manifest = manifest();
    let Some(line) = manifest
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("schemars"))
    else {
        unreachable!("the manifest declares a `schemars` dependency");
    };
    assert!(
        !line.contains("optional = true"),
        "schemars is permanent (got `{line}`)"
    );
    assert!(
        !manifest.contains("schema ="),
        "the `schema` feature must be gone"
    );
}
