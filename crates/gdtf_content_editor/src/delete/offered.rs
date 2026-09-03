//! Which deletes the screen the author has open offers.

use super::registry::{DeleteEntry, DeleteRegistry};
use crate::mode::{EditorMode, InjurySubTab};

/// The entries the editor offers on `mode`, with `sub_tab` open inside it.
pub(crate) fn entries_offered_on(
    registry: &DeleteRegistry,
    mode: EditorMode,
    sub_tab: Option<InjurySubTab>,
) -> Vec<&DeleteEntry> {
    registry
        .entries()
        .filter(|entry| entry.screen().is_open(mode, sub_tab))
        .collect()
}
