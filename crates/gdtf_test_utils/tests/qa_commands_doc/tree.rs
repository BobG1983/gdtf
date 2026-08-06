use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) const GUIDE: &str = "docs/tooling/qa-commands.md";

pub(crate) const WORKED_EXAMPLE: &str = "crates/gdtf_app/src/dev/net_qa/commands/read/app_phase.rs";

pub(crate) fn repo_root() -> PathBuf {
    std::env::var_os("GDTF_QA_COMMANDS_DOC_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

pub(crate) fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    let Ok(text) = fs::read_to_string(&path) else {
        unreachable!("{relative} must exist at {}", path.display());
    };
    text
}
