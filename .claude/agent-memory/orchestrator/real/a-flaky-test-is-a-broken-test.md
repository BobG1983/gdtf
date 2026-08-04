---
name: a-flaky-test-is-a-broken-test
description: A flaky test is a broken test — fix it to be deterministic, no retries and no "CI noise" excuse; never pin a shipped asset's tunable magnitude.
metadata:
  type: feedback
---

A flaky test is a **bad test**. Fix it to be deterministic. Never excuse a flake as "contention",
"timing", or "CI noise", and never add a retry. Do not pile concurrent heavy cargo jobs and then
blame the resulting failures on load.

**Never pin a shipped asset's tunable magnitude.** A loader test asserts that the file parses and
the field is populated — not that a weapon does 7 damage.

**Why:** the repo already made this binding. `.claude/rules/verification.md` rule 6: "Do not pin
changeable literals in tests. If an ordinary content or tuning edit (new weapon file, renamed stem,
magnitude tweak) turns a test red, the test is pinning a changeable literal — assert the property
instead." Exact filenames, counts and magnitudes belong in content data, not in `assert!`. Pinning a
magnitude means every balance change the user makes breaks the suite, which trains everyone to edit
tests to match whatever the code now does. The one exception is the guard suites under
`crates/gdtf_test_utils/tests/` — they pin repo structure on purpose.

**How to apply:** when a test goes red after a tuning or content edit, fix the test, not the data.
Assert the property (registry non-empty, the field deserializes, the gate waits on the resource).
The design-gate structure lens treats a brittle exact-magnitude assert on tunable data as a
violation (`.claude/agents/design-gate.md:35`).

Related: [[probe-the-app-dont-grep]], [[dont-retune-what-the-user-tuned]].
