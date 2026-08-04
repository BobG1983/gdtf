---
name: pattern-consistency-check-confirms-on-mismatch
description: A consistency check whose early-out returns "OK" when the two values DIFFER — the exact divergence it exists to catch slips straight through it.
metadata:
  type: feedback
---

A guard added to catch "A and B address different things" can carry an early-out that returns
Confirmed/OK when A != B, because the author was thinking of one benign mismatch such as a
placeholder value.

Live example: `crates/gdtf_content_editor/src/net_qa/screenshot/aim.rs:39-41` —
`if *wanted != ***target { return CaptureAim::Confirmed; }`. The capture-aim check returns
Confirmed when the shot source does not equal the present path's target, so a future
divergence between those two — the very defect class the check was added for — is waved
through rather than refused.

**Why:** the check's stated purpose is "the pixels this capture reads are the pixels the
camera writes". An early-out on inequality inverts that for the one input that matters.

**How to apply:** on any new consistency or validation guard, read every early-return and ask
which mutation reaches it. Then check whether some OTHER real-path test covers that mutation.
Here it does: `the_capture_source_names_the_created_target`
(`crates/gdtf_content_editor/src/net_qa/present/test/present.rs:63-80`) pins the source equal
to the created target as a whole value, and the GPU pixel readback in
`crates/gdtf_content_editor/tests/net_qa_editor_screenshot` fails on a black frame. Covered
elsewhere = NOTE, not violation; uncovered = violation.
