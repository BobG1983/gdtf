//! Which families a read answers, and why the Prefab tab is not one of them.

use gdtf_qa_protocol::command::ArgumentFault;

use crate::net_qa::wire::EditorModeNet;

// The one place the Prefab tab is turned away, so no caller has to test for it again.
fn no_content_family() -> ArgumentFault {
    ArgumentFault::new(
        "the Prefab tab is the map canvas and carries no content family, so editor.families \
         cannot narrow to it"
            .to_owned(),
    )
}

/// The families a read answers: every one in order without a filter, or the one named.
/// `Prefab` is refused, because it owns no registry.
pub(super) fn families_to_answer(
    only: Option<EditorModeNet>,
) -> Result<Vec<EditorModeNet>, ArgumentFault> {
    match only {
        None => Ok(EditorModeNet::CONTENT_FAMILIES.to_vec()),
        Some(EditorModeNet::Prefab) => Err(no_content_family()),
        Some(family) => Ok(vec![family]),
    }
}
