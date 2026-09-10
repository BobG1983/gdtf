//! Reading one file for the parameter that writes the battle-setup request.

use std::{fs, path::Path};

/// The message only the one generation route writes.
pub(crate) const MESSAGE: &str = "SetupBattleRequested";

const WRITER: &str = "MessageWriter";

/// The system parameter this guard allows in exactly one file.
pub(crate) fn needle() -> String {
    format!("{WRITER}<{MESSAGE}>")
}

/// Whether `path` holds the needle. A file that is not UTF-8 holds nothing.
pub(crate) fn writes_the_request(root: &Path, path: &str) -> bool {
    fs::read_to_string(root.join(path)).is_ok_and(|text| text.contains(&needle()))
}
