---
name: pattern-new-precheck-invalidates-fixtures
description: A new pre-check in front of an action makes every old fixture look like the failure state, forcing a rewrite that can silently drop old assertions — audit the rewritten assertions, not just the new ones.
metadata:
  type: feedback
---

When a diff adds a PRE-CHECK in front of an existing action, every fixture that already set up the
"already there" state now trips the new check. The implementer has to rewrite those fixtures, and
that rewrite is where old coverage quietly disappears.

**Why:** an orphan probe was added ahead of the spawn in `HostManager::launch`, which now returns
`LaunchFailure::PortHeldByOrphan` (`bins/gdtf_qa_mcp/src/lifecycle/manager.rs:145`). The fake QA
listener answered from the moment it bound, so every pre-existing launch test would have failed as
`PortHeldByOrphan`. The fix was a gated listener that only starts answering once the spawner runs
(`FakeGameGate` at `bins/gdtf_qa_mcp/tests/lifecycle/support.rs:72`, `spawn_gated_fake_game` at
`:82`), plus moving `stop()` follow-up assertions onto `free_port()` (`support.rs:141`).

**How to apply:** for each rewritten test, ask what the ORIGINAL assertion pinned and whether the
rewrite still pins it. Here `a_free_port_still_answers_not_running`
(`bins/gdtf_qa_mcp/tests/lifecycle/orphan.rs:197`) ends in
`assert_eq!(manager.stop(port), StopOutcome::NotRunning)` (`:209`), which still pins that a
completed stop cleared `running`, because `stop` tries `stop_owned` first — so it survived. A
rewrite that moved the assertion onto a different manager, or dropped it, would have been a
contract-mandated coverage loss (see [[pattern-contract-mandated-coverage-loss]]).
