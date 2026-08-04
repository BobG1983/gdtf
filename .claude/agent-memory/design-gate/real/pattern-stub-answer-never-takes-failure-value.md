---
name: pattern-stub-answer-never-takes-failure-value
description: A test double with a configurable outcome field is only as good as the values its call sites pass; if all of them pass success, the failure arm of the code under test is dead.
metadata:
  type: feedback
---

A test double built with a configurable outcome field (`struct Watch { answer: OrphanStop,
… }`) proves nothing about the arms its call sites never select. Grep every construction: if
all of them pass the same success value, the production `match`'s other arm can be flipped to
the wrong outcome with the whole suite green.

**How to apply:** on any diff adding a trait double with an outcome or answer field, run
`grep -n "<EnumName>::" tests/` and count the DISTINCT values actually passed, not the number
of tests. Run the same check for a production helper that shells out (kill, lsof): grep the
test tree for calls to the REAL implementation's method, not just for the type name — a type
imported only to call `inspect` proves nothing about `stop`. A render test that hand-builds
the failure value and formats it is not coverage either; the manager has to produce it.

**Both halves, done right, are in the tree** — use them as the picture of adequate coverage.
`bins/gdtf_qa_mcp/tests/lifecycle/orphan.rs:80`
(`an_orphan_that_survives_its_stop_is_reported_as_held`) sets `answer: OrphanStop::Survived`
at `:87` and asserts the manager itself yields `OrphanHeld` at `:94`, which is the only thing
that exercises `lifecycle/manager.rs:88`
(`OrphanStop::Survived => StopOutcome::OrphanHeld`). The real `SystemOrphanWatch::stop` is
covered separately against processes the tests spawn —
`tests/lifecycle/system_watch/escalation.rs:24`, `group_stop.rs:36`, `signal_target.rs:23,39`
— because the double can never prove the signal handling.

Related: [[pattern-mechanism-shipped-instead-of-evidence]],
[[pattern-conjunction-predicate-half-pinned]].
