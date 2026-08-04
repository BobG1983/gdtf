---
name: pattern-contract-mandated-coverage-loss
description: A clause that dictates a test's SETUP can delete the only proof of a neighbouring property — name what stops being covered, but rule it a note, not a violation.
metadata:
  type: feedback
---

When a clause dictates a test's setup rather than its assertions, compare what the old setup
covered by accident against what the new one covers. Re-homing a test can delete the only proof
of a property nobody listed. Name it, and rule it a note — blocking work for obeying its own
ticket is how a gate loses credibility.

**Why:** the editor's QA listener has to answer a handshake in every editor state, including
during the asset load pass, or a client that connects early is dropped by its own read timeout
(`crates/gdtf_net_qa_transport/tests/transport/reap.rs:12` is the test that an idle client is
reaped). A test that happened to sit in `Load` because its app never advanced was once the only
thing proving it. Driving that test to `Editing` — which a clause required — took the proof away
without touching a single assertion, and nothing went red.

The fix is what the property deserved anyway: a deliberate case instead of an accident.
`crates/gdtf_content_editor/tests/net_qa_hello/load_case.rs:14-37` asserts the editor is still
in `EditorState::Load` when the request goes out (`:27-32`), and `client.rs:77-91` checks the
handshake is answered before the measured request is even sent (`:84-87`). Three tests ride
that helper — the handshake, a version mismatch, and the editor's own drain
(`crates/gdtf_content_editor/tests/net_qa_hello/main.rs:43-69`).

**How to apply:** name the surviving change explicitly ("branch this on state — no test fails"),
cite whatever states the property matters (a production doc line, or the test that pins it now),
and rule it a note when the clause's own words forced the setup. Suggest the follow-up ticket
rather than blocking. An accidental proof is worth converting into a deliberate one.

Related: [[pattern-canned-fixture-carries-the-unasserted-value]].
