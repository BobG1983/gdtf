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

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .components()
        .collect()
}

fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    let Ok(text) = std::fs::read_to_string(&path) else {
        unreachable!("{relative} is readable at {}", path.display());
    };
    text
}

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

const QUALIFIED_SCHEMA_FEATURE: &str = "gdtf_qa_protocol/schema";

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
