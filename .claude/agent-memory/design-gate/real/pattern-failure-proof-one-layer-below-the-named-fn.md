---
name: pattern-failure-proof-one-layer-below-the-named-fn
description: A "prove the assertion FAILS" clause answered by driving the inner check_* helper to a failing verdict, while the named assert_* wrapper is only ever run on passing input.
metadata:
  type: feedback
---

A clause of the form "`assert_X` has a test proving it FAILS on broken input" is NOT met by a
test that drives a lower-layer `check_X(rows) -> Verdict` to its failure variant. The
wrapper is the thing the clause names, and it stays un-exercised on the failure side —
delete its `assert_eq!` and the suite stays green.

**Why:** the usual defence is "the broken input cannot be built", and it is usually wrong,
because the assertion takes a TRAIT OBJECT. In `crates/gdtf_qa_command`,
`assert_unique_names` (`src/test_support/assert.rs:101`) and `assert_schemas_parse` (`:115`)
both take `&[&dyn ErasedCommand<F>]`, and the crate's only production impl is the blanket
`impl<C: QaCommand> ErasedCommand<C::Facts> for C` (`src/command/erased.rs:29`). Coherence
still lets a test-local type that does NOT implement `QaCommand` hand-implement
`ErasedCommand` and return the broken value — so the failing-side test is reachable and the
clause is satisfiable.

The tree now proves it. `FakeBrokenSchema` hand-implements `ErasedCommand<FakeFacts>`
(`src/test_support/fake/broken.rs:16`) and publishes an argument document that is not JSON
(`:11`, `:29-31`). Both wrappers now have real failing-side tests over `catch_unwind` —
names at `src/test_support/test/assert.rs:29-38`, schemas at `:97-106` — alongside the
inner-layer `check_schemas_parse` tests (`assert.rs:78`, exercised at `test/assert.rs:56-95`)
that used to stand in for them.

**How to apply:** find the escape hatch before accepting a "cannot be driven" argument. If
the input is `&[&dyn Trait]` and the only impl is blanket over a second trait, a test-local
type that skips the second trait can implement the first directly. If the input is a
concrete type with a private constructor, the argument may hold — say so explicitly rather
than accepting the lower-layer test in silence.

Related: [[pattern-conjunction-predicate-half-pinned]].
