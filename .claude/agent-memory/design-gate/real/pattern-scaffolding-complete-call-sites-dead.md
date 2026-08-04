---
name: pattern-scaffolding-complete-call-sites-dead
description: The whole supporting cast of a fix can land — module, enum variants, render arms, tool text, docs, tests — while the one call site still takes the old path. Grep for callers of each new helper.
metadata:
  type: feedback
---

A diff can look complete from the outside — new module, new enum variants, new render arms,
updated tool descriptions, updated `docs/`, a full test suite asserting the new behaviour —
while the ONE place that has to call the new machinery still does the old thing. Never infer
behaviour from the surrounding scaffolding.

**How to apply:** for every new private helper the fix adds, run `grep -rn "<helper>" src
tests` and count call sites separately from the definition. Zero callers means the clause is
unbuilt, and under `-D warnings` the suite is red on `dead_code`. Do the same for an
argument a signature gained: `let _ = <arg>;` in the body is the shape of a discarded
parameter. A doc comment describing the check is not the check — read the statements between
the guard and the return.

**Corollary: you can rule the suite RED without running cargo.** Read what the tests
destructure and confirm no production path produces that variant. Two independent proofs of
redness follow from one reading — the failing destructure, and `dead_code` on the uncalled
helpers. This matters when the verify agent dies and no suite evidence exists.

**The orphan-port work is the worked example, now in its FIXED state** — use it as the
picture of what "wired" looks like. `bins/gdtf_qa_mcp/src/lifecycle/orphan.rs`, the outcome
variants (`lifecycle/outcome.rs:52` `PortHeldByOrphan`, `:69` `OrphanStopped`, `:76`
`OrphanHeld`), and the tests (`tests/lifecycle/orphan.rs`, `production_wiring.rs`) all
exist, AND the helpers have real callers: `lifecycle/manager.rs:144`
`if let PortHold::Orphan(pid) = self.hold_on(port)` reaching `hold_on` at `:77`, and `:162`
`PortHold::Orphan(pid) => self.stop_orphan(port, pid)` reaching `stop_orphan` at `:81`.
`stop` takes the port and uses it (`:155-163`) instead of discarding it. The failure this
memory records was that same tree with every one of those callers missing.

Related: [[pattern-mechanism-shipped-instead-of-evidence]],
[[pattern-test-doc-overclaims-its-assertion]].
