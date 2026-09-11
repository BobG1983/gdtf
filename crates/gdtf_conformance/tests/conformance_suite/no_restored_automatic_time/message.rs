//! The failure text the guard prints when a tracked Rust file hands the frame clock back.

use crate::no_restored_automatic_time::scan::{TYPE_NAME, needle};

/// The failure text for `lines`, the `path:line` entries that hand the frame clock back.
pub(crate) fn restored_clock_message(lines: &[String]) -> String {
    format!(
        "these lines write {}, so every frame after them reads whatever the machine took:\n\
         {}\n\nDelete the write. A test that pins {TYPE_NAME}::ManualDuration leaves it \
         pinned, and a helper that steps time by hand returns with a manual delta still \
         pinned: either the step it used, or the pinned delta its harness starts from. The \
         policy is in docs/testing.md.",
        needle(),
        lines.join("\n"),
    )
}

#[test]
fn the_failure_text_lists_every_offending_line_and_says_what_the_policy_doc_says() {
    let lines = [
        "crates/gdtf_game/tests/game_suite/one.rs:12".to_owned(),
        "libs/cobalt_test_utils/src/two.rs:34".to_owned(),
    ];

    let message = restored_clock_message(&lines);

    for line in &lines {
        assert!(
            message.contains(line),
            "the failure text must list {line}, or the reader cannot find the write:\n{message}"
        );
    }
    assert!(
        message.contains(&needle()),
        "the failure text must name {}, the variant this guard forbids:\n{message}",
        needle()
    );
    assert!(
        message.contains("docs/testing.md"),
        "the failure text must point at docs/testing.md, which holds the policy:\n{message}"
    );
    assert!(
        !message.contains("the next helper pins its own step"),
        "docs/testing.md dropped that wording, because the effects helpers restore a shared \
         pinned delta rather than a step of their own:\n{message}"
    );
}
