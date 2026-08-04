---
name: pattern-conjunction-predicate-half-pinned
description: A predicate over K optional fields tested only with the all-absent and all-present values pins none of its terms — demand K+1 assertions.
metadata:
  type: feedback
---

A predicate over K optional fields tested with only the all-absent and all-present values pins
nothing about the individual terms: either half can be deleted and both assertions still pass.
Demand K+1 assertions — the empty case plus one per field.

**Why:** `RunOptions::is_plain` (`crates/gdtf_qa_protocol/src/command/options.rs:57-59`) is
`await_ready.is_none() && capture.is_none()`. Asserting it on `RunOptions::default()` and on a
both-riders-set value exercises both halves together, so dropping either conjunct leaves the
test green.

`crates/gdtf_qa_protocol/src/command/test/round_trip.rs:153-181` is the shape to ask for: the
default is plain (`:156`), both-set is not (`:165`), await-only is not (`:169-172`), and
capture-only is not (`:176-179`). With those four, neither conjunct can be removed while green.

**How to apply:** for any new boolean over K optional or flag fields, count the assertions. The
tell is a test that builds single-field values for some other purpose — a round trip, a
serialization check — and never runs the predicate over them. Name the conjunct that is
deletable.

Related: [[pattern-canned-fixture-carries-the-unasserted-value]].
