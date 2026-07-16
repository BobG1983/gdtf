//! Compile-level guarantee that `gdtf_qa_protocol` is ENGINE-FREE (GTW-734).
//!
//! The crate must link WITHOUT the Bevy engine so both the game-side `net_qa` server
//! (T3) and the standalone `gdtf_qa_mcp` bridge (T8) can depend on it. This test asserts
//! the dependency LIST directly: the only Bevy edge permitted is the `bevy_derive`
//! proc-macro crate (the `Deref` derive), and no async runtime (`tokio`) or the Bevy
//! ENGINE (`bevy`) may appear. The `/gate` structure lens re-checks the same fact with
//! `cargo tree`.

/// Read this crate's own `Cargo.toml`.
fn manifest() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let Ok(text) = std::fs::read_to_string(path) else {
        unreachable!("the crate's own Cargo.toml is readable at {path}");
    };
    text
}

/// The dependency NAMES declared under `[dependencies]` (the runtime deps), in order.
fn runtime_dependency_names(manifest: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            // A new table header — we are inside `[dependencies]` only for that exact
            // table (never `[dev-dependencies]`, `[lints]`, etc.).
            in_deps = trimmed == "[dependencies]";
            continue;
        }
        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // A dependency line is `name = ...` — take the token before `=`.
        if let Some((name, _)) = trimmed.split_once('=') {
            names.push(name.trim().to_owned());
        }
    }
    names
}

/// The runtime dependency list carries the derive crate, never the Bevy engine or an
/// async runtime.
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
    // The whole runtime dependency set is exactly the three the contract allows.
    for name in &names {
        assert!(
            matches!(name.as_str(), "serde" | "ron" | "bevy_derive"),
            "unexpected runtime dependency `{name}` (allowed: serde, ron, bevy_derive)"
        );
    }
}
