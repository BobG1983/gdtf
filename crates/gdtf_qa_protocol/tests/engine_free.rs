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
    // The whole runtime dependency set is exactly the four the contract allows.
    for name in &names {
        assert!(
            matches!(name.as_str(), "serde" | "ron" | "bevy_derive" | "schemars"),
            "unexpected runtime dependency `{name}` (allowed: serde, ron, bevy_derive, schemars)"
        );
    }
}

/// The repository root, two directories above this crate's manifest.
fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .components()
        .collect()
}

/// Read a repo-relative file, failing loudly if it has moved.
fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    let Ok(text) = std::fs::read_to_string(&path) else {
        unreachable!("{relative} is readable at {}", path.display());
    };
    text
}

/// `schemars` is OPTIONAL and reached only through the `schema` feature (GTW-939).
///
/// The courier (`bins/gdtf_qa_mcp`) carries schema documents as opaque text and must never
/// link a schema library; the two hosts turn the feature on to derive their command
/// argument / reply schemas. If the dependency ever loses `optional = true`, or the feature
/// stops naming it, every consumer starts linking schemars — which this pins against.
#[test]
fn schemars_is_optional_and_behind_the_schema_feature() {
    let manifest = manifest();

    let Some(line) = manifest
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("schemars"))
    else {
        unreachable!("the manifest declares a `schemars` dependency");
    };
    assert!(
        line.contains("optional = true"),
        "the schemars dependency must stay optional (got `{line}`)"
    );
    assert!(
        manifest.contains("schema = [\"dep:schemars\"]"),
        "the `schema` feature must be the only way to reach schemars"
    );
}

/// The courier does not turn the `schema` feature on (GTW-939).
///
/// `bins/gdtf_qa_mcp` moves schema documents as opaque text; it has no reason to derive
/// one, and a `features = ["schema"]` on its dependency line would make it link schemars on
/// every build of the courier.
#[test]
fn the_courier_does_not_enable_the_schema_feature() {
    let text = read_repo_file("bins/gdtf_qa_mcp/Cargo.toml");
    let Some(line) = text
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("gdtf_qa_protocol"))
    else {
        unreachable!("the courier declares a `gdtf_qa_protocol` dependency");
    };
    assert!(
        !line.contains("schema"),
        "the courier must not enable the `schema` feature (got `{line}`)"
    );
}

/// The package-qualified feature name that turns `schema` on from outside this crate.
const QUALIFIED_SCHEMA_FEATURE: &str = "gdtf_qa_protocol/schema";

/// The build-configuration files that can turn a package feature on for a whole-workspace
/// command: the shared cargo aliases and every CI workflow.
fn workspace_command_files() -> Vec<(String, String)> {
    let mut files = vec![(
        ".cargo/config.toml".to_owned(),
        read_repo_file(".cargo/config.toml"),
    )];
    let dir = repo_root().join(".github/workflows");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        unreachable!("the workflow directory is readable at {}", dir.display());
    };
    let mut workflows: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_yaml = std::path::Path::new(&name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml"));
        if is_yaml {
            workflows.push(name);
        }
    }
    workflows.sort();
    assert!(
        !workflows.is_empty(),
        "no workflow files found under {} — this guard would be vacuous",
        dir.display()
    );
    for name in workflows {
        let relative = format!(".github/workflows/{name}");
        let text = read_repo_file(&relative);
        files.push((relative, text));
    }
    files
}

/// No whole-workspace cargo command enables `schema` (GTW-939).
///
/// The courier links schemars whenever the one shared `gdtf_qa_protocol` library unit is
/// built with `schema` on, and cargo builds exactly one such unit per invocation. So naming
/// `gdtf_qa_protocol/schema` on ANY `--workspace` command — a `.cargo/config.toml` alias or
/// a CI workflow step — puts schemars in the courier, no matter what the courier's own
/// manifest says. The feature belongs on crate-scoped runs
/// (`cargo test -p gdtf_qa_protocol --features schema`) and on the two hosts, which do not
/// build the courier. This walks every line that carries `--workspace` in those files and
/// fails if one names the feature.
#[test]
fn no_workspace_wide_command_enables_the_schema_feature() {
    let mut violations: Vec<String> = Vec::new();
    let mut workspace_lines = 0_usize;
    for (file, text) in workspace_command_files() {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || !trimmed.contains("--workspace") {
                continue;
            }
            workspace_lines += 1;
            if trimmed.contains(QUALIFIED_SCHEMA_FEATURE) {
                violations.push(format!(
                    "{file} — `{trimmed}` names `{QUALIFIED_SCHEMA_FEATURE}`; that builds the \
                     one shared gdtf_qa_protocol unit with schema on, so bins/gdtf_qa_mcp links \
                     schemars"
                ));
            }
        }
    }
    assert!(
        workspace_lines > 0,
        "no `--workspace` command found in the alias or workflow files — this guard would be \
         vacuous"
    );
    assert!(
        violations.is_empty(),
        "workspace-wide commands must not enable `schema`:\n{}",
        violations.join("\n")
    );
}
