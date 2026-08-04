---
name: pattern-injected-double-leaves-production-wiring-unpinned
description: Every test for the new behavior injects a double through a test-only constructor, so the one line wiring the real collaborator into the production constructor is asserted by nothing.
metadata:
  type: feedback
---

When a fix adds a trait, a real impl, and a test-only `with_<collaborator>` constructor, check
whether any test builds the object through the PRODUCTION constructor and exercises the new
behavior. If none does, swapping the real impl for an inert one in that constructor brings the
whole defect back with a green suite.

**Why:** `HostManager::with_config` is the constructor the shipped host calls, and its whole
contribution to the orphan check is one line —
`Self::with_orphan_watch(spawner, config, Box::new(SystemOrphanWatch::new()))`
(`bins/gdtf_qa_mcp/src/lifecycle/manager.rs:58-59`). Every orphan test built through
`with_orphan_watch` with a double, so that line was revertible.

**How to apply:** grep the constructor call sites (`grep -rn "with_config\|with_<x>" src tests`).
Then ask whether a production-constructor test is genuinely unsafe or merely wasn't written — the
launch path only reports the orphan and signals nothing, so it was always safe to drive through
`with_config`. The shape that closes it is
`bins/gdtf_qa_mcp/tests/lifecycle/production_wiring.rs`:
`the_production_constructor_sees_a_held_port` (`:8`) builds through `with_config` against a really
held port and expects `LaunchFailure::PortHeldByOrphan`, and
`the_production_constructor_still_launches_on_a_free_port` (`:25`) keeps that pin from degrading
into "every launch fails".

Check the render arms per enum variant too — the arm that fires in the reported scenario is the
one most often left unrendered. Both are asserted here: `OrphanPid::Known` at
`bins/gdtf_qa_mcp/src/mcp/control/test/launch.rs:188,224` and `test/render.rs:121`,
`OrphanPid::Unknown` at `launch.rs:207`.
