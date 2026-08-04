---
name: pattern-deferred-act-has-no-refusal-path
description: A Deferred act command that settles by watching the ActLog has no answer for "the sim refused" — the sim refuses silently, so the parked reply burns its budget and answers Timeout instead of a typed refusal.
metadata:
  type: feedback
---

A `CommandTiming::Deferred` act command that settles by watching the `ActLog` has no answer for
"the sim refused". The offer gate in `gdtf_app` is advisory; the sim's dispatch is
authoritative and refuses by `continue` — no rejection message, no log entry. Every refusal in
`crates/gdtf_battle_sim/src/acts/shove/dispatch.rs` is one of those: `:98` and `:103` (either
ganger missing), `:111` (not adjacent, same faction, or target already down), `:114` (no
time-unit component), `:124` (target gone before the outcome applies).

The parked reply then sits in `DeferredReplies<C>` until it expires at `DEFERRED_BUDGET` = 2 s
(`crates/gdtf_qa_command/src/dispatch/deferred.rs:25`, expiry in `sweep_expired` at `:145-165`,
driven by `sweep_deferred` at `:169-174`, registered for EVERY command at
`crates/gdtf_qa_command/src/dispatch/register.rs:21`). The client gets
`QaResponse::Error(QaError::Timeout)`, not a typed refusal. The same hole opens when the act
passes both gates but changes nothing observable — a shove whose displacement is blocked writes
no `Position`, so the query-sourced recorders log nothing.

**Why:** the sim publishes no rejection vocabulary at all, so a command's refusal enum has
nothing to mirror. No `Deferred` act command exists yet — the only `CommandTiming::Deferred` in
the tree is the fake at `crates/gdtf_qa_command/src/test_support/fake/settle.rs:90` — so this
is a trap to catch at contract-audit time, not a live defect.

**How to apply:** on any ticket adding a `Deferred` act command, ask what answers the parked
responder when the act does not resolve. Demand either a bounded settle (N frames, then a typed
refusal) or an explicit ruling that `Timeout` is the accepted answer. Do not let a clause
covering the playback gate stand in for it.

Related: [[unavailable-command-never-reaches-claim]].
