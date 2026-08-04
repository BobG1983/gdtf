---
name: pattern-unsatisfiable-half-built-and-disclosed-late
description: A clause that is provably impossible in part gets built to its satisfiable subset and disclosed in the report instead of amended on the ticket — still a violation.
metadata:
  type: feedback
---

A clause whose remainder is provably impossible gets built to the part that works, with the
reason written into a module doc and a "deviation" note in the report. That is still a
violation.

**Why:** `design-fidelity.md` rule 2 requires approval BEFORE building, rule 4 requires
surfacing a ticket-vs-`docs` conflict instead of picking a side, and the descoping protocol is
explicit — STOP, present the tradeoff, amend the TICKET first, only then code. A report written
after the build is disclosure, not approval, and it leaves the landed tree disagreeing with the
ticket a future reader will audit against.

**How to apply:** when a clause is partly impossible, check three things before ruling.
(1) Is the impossibility real? Trace it — a type that is a FIELD of another crate's shipped
struct, or a doc link whose target the same change deletes, cannot move or be re-pathed.
(2) Was it amended on the ticket, by a description edit or a comment carrying a user ruling? A
pre-build comment that corrects OTHER premises and stays silent on this one is evidence it was
NOT ruled. (3) Does the design spec disagree with the clause? All three true → VIOLATION: send
it back for a one-line ticket amendment, and do not credit the code for doing the sensible
thing.

Seen on a "move `src/ids/` into `wire/`" clause: `crates/gdtf_qa_protocol/src/ids/cell.rs` and
`ids/shot.rs` are still in the protocol crate while the game's copy lives in
`crates/gdtf_app/src/dev/net_qa/wire/cell.rs`, and 13 of 32 doc links became prose.

Related: [[pattern-unsatisfiable-acceptance-clause]],
[[pattern-two-types-same-name-after-vocabulary-move]].
