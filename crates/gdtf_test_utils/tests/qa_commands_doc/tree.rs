use std::{fs, path::PathBuf};

pub(crate) const GUIDE: &str = "docs/tooling/qa-commands.md";

pub(crate) const WORKED_EXAMPLE: &str = "crates/gdtf_app/src/dev/net_qa/commands/read/app_phase.rs";

pub(crate) fn repo_root() -> PathBuf {
    if let Some(override_root) = std::env::var_os("GDTF_QA_COMMANDS_DOC_ROOT") {
        return PathBuf::from(override_root);
    }
    let Some(root) = gdtf_assets::workspace_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

pub(crate) fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    let Ok(text) = fs::read_to_string(&path) else {
        unreachable!("{relative} must exist at {}", path.display());
    };
    text
}
