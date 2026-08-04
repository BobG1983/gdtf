---
name: pattern-dead-assertion-arity-drift
description: A slice-pattern destructure whose arity drifted from the producer makes every assertion after it dead code — count the producer's elements against the pattern before crediting any assertion.
metadata:
  type: feedback
---

When a test collects replies into a `Vec` and the assertion fn destructures them with a
fixed-length slice pattern (`let [a, b, c] = replies else { unreachable!(...) }`), COUNT the
producer's elements against the pattern's arity before crediting ANY clause satisfied by an
assertion below that line.

**Why:** the arity check is the first statement in the fn, so an added exchange with an
untouched pattern makes the whole rest of the file unreachable. The values written there can
be correct and still never be evaluated. A summary claiming "20 sequential + 72 concurrent
passing runs" once got pasted against a tree that panicked 100% of the time. Two cheap tells
that the edit was incomplete: the assertion fn's doc comment still names the old count while
the client's names another, and a request kind is sent by the client but appears in no match
arm on the assertion side. A clause whose corrected assertion sits below a failing arity guard
is a violation, not a partial pass — the same "unreachable code satisfies a clause" failure as
an unwired Bevy system.

**The fix shape that closes the class**, live in
`crates/gdtf_content_editor/tests/net_qa_hello/`: put the SAME fixed-size array type on both
halves. `EDITING_EXCHANGES = 3` is one shared const (`support.rs:16`), the collector
`try_into()`s its `Vec` and reports a short read as an error instead of panicking
(`drive.rs:17-26`), and the test destructures the array (`main.rs:36`). An exchange added
without an assertion arm then fails to COMPILE.

**How to apply:** on any test-editing ticket, diff the producer and the consumer in the same
pass. Credit an assertion clause only when the consumer's arity is compiler-enforced or an
explicit `assert_eq!(rest.len(), …)` guards it.

Related: [[pattern-partial-falsehood-sweep]].
