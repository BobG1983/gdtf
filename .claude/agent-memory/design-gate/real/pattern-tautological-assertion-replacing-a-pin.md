---
name: pattern-tautological-assertion-replacing-a-pin
description: An assertion whose expected value comes from the same place as the actual one cannot fail — read the producer and ask what value could reach the assert and break it.
metadata:
  type: feedback
---

For every assertion a contract clause leans on, open the code that PRODUCES the value and ask
"what could reach this line and fail it?" If the answer is "nothing", the test is decoration.
Two shapes to watch for: comparing two fields the producer built from one binding, and
`assert!(matches!(x, A | B))` where the producer emits only `A` and `B`.

**Why:** these read as rigorous, usually carry a justifying comment, and survive every
mutation probe aimed at the assertions around them — so a reviewer who only re-runs the
implementer's probes misses them. They usually appear when a real pin is loosened to stop a
timing flake.

**How to apply:** trace the expected value back. It has to come from somewhere the producer
could not have produced. `bins/gdtf_qa_mcp/tests/jsonrpc/host_local.rs:64-81` is the shape
that works: it asks for the editor's logs, asserts the lines equal `EDITOR_LOG` (`:71-75`) and
then asserts they do NOT equal `GAME_LOG` (`:76-80`). Those two expectations are separate
consts with different text (`tests/jsonrpc/support.rs:133-134`), handed to two different
fixtures at `:177-186`. Make the two consts hold the same string and the `assert_eq!` passes
no matter which lifecycle `src/mcp/courier/handle.rs:65` reached — same assertion, zero
discrimination. The fixture, not the assert, is what makes it able to fail.

When you find one, report it with the producer's file:line even if the fact is pinned properly
somewhere else; staying silent lets the weakening compound in the next ticket.

Related: [[pattern-third-sibling-skips-the-host-discrimination]],
[[pattern-test-doc-overclaims-its-assertion]].
