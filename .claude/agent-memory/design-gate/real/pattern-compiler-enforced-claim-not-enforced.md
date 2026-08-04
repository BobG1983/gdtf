---
name: pattern-compiler-enforced-claim-not-enforced
description: Read every "cannot" / "the only place" / "compiler-enforced" claim in a doc as testable, then grep for the public API that reaches the value another way.
metadata:
  type: feedback
---

Check every claim written as an absolute — "cannot", "the only place", "compiler-enforced" —
by grepping for a public API that reaches the forbidden value another way. The claim is
usually true of the wired path and false as a type-level guarantee.

**Why:** the QA command layer holds the property that a handler sees typed arguments rather
than raw JSON or a raw responder, and it holds because of how the plugin wires the systems,
not because the types prevent anything. `CommandInbox` is a public Bevy resource
(`crates/gdtf_qa_command/src/dispatch/inbox.rs:24`) whose public `take_for` (`:50`) hands back
`Vec<(CommandArgsJson, Responder)>`. `PendingQueue::drain_ready`
(`crates/gdtf_net_qa_transport/src/pending/queue.rs:51`) is public and returns a raw
`Responder`. `NetInbox::drain` (`crates/gdtf_net_qa_transport/src/channel/inbox.rs:22`) is
public too. Any system already holding the resource it needs can take all three.

The pins that do exist say what they are.
`crates/gdtf_app/tests/net_qa/command_set.rs:34-84` asserts "exactly one place in the game may
drain the inbox" by reading the source files under `src/dev/net_qa` and counting
`inbox.drain()` sites, then checks the router system is registered once. That is a claim about
this crate's code, checked as such — not a claim about the type system.

**How to apply:** rule it a finding when the doc is the contract the next ticket's authors
build against. Ask for "does not, on the path this crate wires" instead of "cannot". The grep
for the escape hatch takes a minute and either confirms the wording or replaces it.

Related: [[pattern-conjunction-predicate-half-pinned]].
