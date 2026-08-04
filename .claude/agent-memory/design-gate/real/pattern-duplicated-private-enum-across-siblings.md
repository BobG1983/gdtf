---
name: pattern-duplicated-private-enum-across-siblings
description: A new sibling module copies a private helper enum verbatim instead of reaching for the family's shared values module — a drift hazard worth a note, but NOT a citable module-layout violation.
metadata:
  type: feedback
---

When a change adds a module beside existing ones in the same directory, diff the new file's
private helper types against its siblings'.

**Why:** `enum StopSignal` plus `impl StopSignal { const fn flag(self) -> &'static str }` is
declared twice today — `bins/gdtf_qa_mcp/src/lifecycle/orphan.rs:155-168` and
`bins/gdtf_qa_mcp/src/lifecycle/child.rs:47-60` — while the same family already has a shared
`bins/gdtf_qa_mcp/src/lifecycle/values.rs` holding `ChildPid` (:7), `PollInterval` (:136),
`KillGrace` (:156) and `ProbeTimeout` (:176). Change `-TERM` to `-INT` in one copy and the
other diverges silently: nothing fails to compile and no test compares them.

**How to apply:** report it as a NOTE, not a block. `.claude/rules/module-layout.md` rule 6
("helpers with 2+ consuming modules live in the shared support/harness module", :54-55) sits
under the heading "Splits are behavior-preserving pure moves" (:48), so it governs SPLIT
operations. Stretching it to cover a brand-new module is an over-reach, and a false block
costs a whole gate round. Name both file:line ranges, name the shared module the type
belongs in, and let the fidelity lens or a follow-up ticket decide.

The near neighbour that IS citable: a new sibling that hard-codes a `Duration` instead of
using the family's existing newtype. That one is a straight `no-bare-types.md` rule 1
violation, because the rule names `Duration` explicitly — same "did not reach for what the
family already has" shape, different verdict.

Related: [[pattern-newtype-stripped-to-serve-two-callers]].
