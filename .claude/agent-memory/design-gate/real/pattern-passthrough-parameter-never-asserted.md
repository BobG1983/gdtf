---
name: pattern-passthrough-parameter-never-asserted
description: A parameter a builder forwards straight into a constructor usually reaches no assertion, so replacing it with a constant keeps the suite green.
metadata:
  type: feedback
---

When a clause dictates a signature, check that EVERY parameter reaches an assertion. The
interesting argument gets a test; the pass-through one usually does not.

**Why:** `crates/gdtf_qa_command/src/catalogue/build.rs:12-16` takes `host: ServerNameNet`,
never touches it, and hands it to `CommandCatalogue::new(host, entries)` at `:30`. Every
test builds the catalogue with `fake_host_name()` and asserts only on entries
(`tests/command_set/growth.rs:18`, `one_predicate.rs:14`); grepping
`crates/gdtf_qa_command/tests` for `host`, `ServerNameNet` or `server_name` returns nothing.
Hardcoding a different name inside `catalogue()` stays green.

Same shape in the MCP host: `bins/gdtf_qa_mcp/src/link.rs:120` `with_timeout` has exactly
one caller — `new` at `:115`, passing the `LINK_TIMEOUT` const — so nothing proves the field
is read. The transport twin IS discriminated:
`crates/gdtf_net_qa_transport/tests/transport/reap.rs:13` passes 150 ms against the 5 s
`DEFAULT_IO_TIMEOUT` (`crates/gdtf_net_qa_transport/src/config.rs:8`). That is what a real
pin looks like.

**How to apply:** grep the test tree for the field the parameter lands in (`.host`, `.name`,
whatever the constructor stores) AND for any non-default value being passed. No hit means an
unasserted pass-through. NOTE when no clause names that value's behaviour, VIOLATION when
one does.

Related: [[pattern-mirrored-unasserted-plumbing-consts]],
[[pattern-canned-fixture-carries-the-unasserted-value]].
