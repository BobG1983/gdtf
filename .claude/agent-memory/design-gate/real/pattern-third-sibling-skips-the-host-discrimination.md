---
name: pattern-third-sibling-skips-the-host-discrimination
description: A new arm in a dispatch match that takes a routing argument needs its own per-route test AND a fixture whose values differ per route — check the fixture, not just the test list.
metadata:
  type: feedback
---

When a ticket adds an arm beside siblings that already prove "the routing argument selects
which child", check two things: the new arm gets its own case at a non-default route, and the
shared fixture carries DIFFERENT values per route for the thing that arm returns.

**Why:** `bins/gdtf_qa_mcp/src/mcp/courier/handle.rs` resolves `host` once (`:54-57`, parsed
by `resolve_host` at `:17-29`), looks up one pair (`:58`), and hands it to every arm of the
match (`:59-68`). An arm that ignored the pair and reached for the game would leave the whole
suite green — the reply even carries `"host"`, so a wrong lookup comes back labelled with the
host the caller ASKED for.

**The fixture is the tell.** `bins/gdtf_qa_mcp/tests/jsonrpc/support.rs` gives the two canned
lifecycles distinct ports and pids (`:136-139`) and distinct log text (`GAME_LOG` `:133`,
`EDITOR_LOG` `:134`), wired at `:177-186`. Every value a tool returns has to differ per host
or no test of that tool can discriminate. With that in place the three arms each have a real
case: `tests/jsonrpc/host_local.rs` proves `launch` (`:19-32`), `stop` (`:35-46`) and `logs`
(`:64-81`, which also asserts the reply is NOT the game's lines), and the unit suite pins the
label with `QaHost::Editor` at `src/mcp/control/test/logs.rs:76-91`.

**How to apply:** for every arm of a dispatch match that takes a routing argument, grep the
test tree for that tool name plus the non-default routing value. Then open the shared fixture
and check the per-route values cover the new arm's OUTPUT — if they do not, adding a case
still proves nothing.

Related: [[pattern-tautological-assertion-replacing-a-pin]],
[[pattern-second-rider-untested-beside-the-tested-one]].
