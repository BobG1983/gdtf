---
name: pattern-second-rider-untested-beside-the-tested-one
description: When a clause proves ONE optional argument travels end to end, check every sibling parsed in the same function — a dropped optional argument looks exactly like one nobody sent.
metadata:
  type: feedback
---

An acceptance clause that names one optional argument gets a thorough test; the optional
argument beside it gets a schema entry and nothing else. Read the whole parse function, not
the one parser the clause names.

**Why:** every optional-argument parser in `bins/gdtf_qa_mcp/src/mcp/courier/run.rs` returns
`Ok(None)` when the argument is absent — `parse_await_ready` (`:38-46`) and `parse_capture`
(`:49-58`). If a parser drops a value the caller DID send, the result is the same `Ok(None)`,
so nothing downstream can tell the two apart. `parse_run` (`:60-66`) folds four parsers into
one `RunCommand`; a test that only exercises one of them leaves the rest free to swallow
input silently.

**How to apply:** open the function the clause's argument is parsed in and list its siblings.
For each sibling ask which test would go red if its body were replaced by `Ok(None)`. The
current tests are the shape to expect: `bins/gdtf_qa_mcp/tests/jsonrpc/courier_riders.rs`
covers `await_ready` (`:16-31`, asserting the refusal names `await_ready=5`) and all three
`capture` shapes — a stem string and bare `true` (`:34-59`), `false` meaning no rider
(`:62-72`), and a wrong-typed value being an error rather than a dropped argument (`:75-83`).
Each of those asserts a value the host RECEIVED, which is what makes dropping visible.

Related: [[pattern-third-sibling-skips-the-host-discrimination]],
[[pattern-tautological-assertion-replacing-a-pin]].
