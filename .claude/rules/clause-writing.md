# Clause writing: a clause a builder can satisfy and a run can prove

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

The clause audit keeps correcting the same five things, all decided when the ticket was written.
The writer had no list. This is that list.

It applies to anyone writing ticket text: `/file-bug`, `/next-task`, and the `project-manager`
agent. `clause-audit` checks against it.

[linear-discipline.md](./linear-discipline.md) owns how a clause cites things: the symbol or the
quoted text, never a bare line number. This file owns whether the clause can be built and shown.

## Every clause answers three questions

1. What changes? Name the behaviour concretely.
2. Where? Name the symbol and the file that holds it, and check it exists right now.
3. What goes red if it is wrong? Name the test, and the mutation it catches.

A clause missing the third is a wish. "No longer depends on the default order" is true the moment
the ordering edge lands, so nothing can fail. Rewrite it as the case that reads the downed
selection, plus the update count that keeps that case readable.

## The nine things a clause pins down

### Direction, for anything ordered

An ordering clause names both endpoints in order: A before B, not "an edge between them". The two
directions are different tickets, and the wrong one can void another clause.

Measured on GTW-1144. Putting `clear_downed_selection` first would have cleared the selection
before the offer systems read it, the exact case three other clauses existed to pin.

### Which crate owns the edge

Say which crate configures the ordering edge. A set defined in one crate cannot be named by a
crate that does not depend on it, so "add the edge" has exactly one legal home.

### The helper's behaviour

If the assertion goes through a test helper, state the part of that helper's behaviour the clause
depends on. `ordered_before` walks direct edges only, not transitively. An edge attached per
system instead of on the set leaves the assertion false while the edge is present, and the builder
cannot see why from the clause.

### Only things that exist

Do not name a per-item spawn when registration is generic. Open the file and read it before you
write the clause. The audit will, and finding the problem while writing is cheaper.

### A mutation that fails for the stated reason

Naming a mutation is evidence only if the red run is caused by the thing the clause is about. A
second wielded weapon that flips an earlier assertion proves nothing about the guard. Say which
assertion must go red, and rule out the other ways it could.

### Nothing a clause asks for lives off the tree

A clause never says "file a bug." Nor "the ticket exists", nor "the label is off", nor anything
else answered by reading the board. A clause is satisfied by code, and shown by a run. Work that
has to happen first but leaves no trace in the tree is a precondition. File it, and make it a
blocking edge on the ticket.

Measured on GTW-1002. Its clause 7 read "The editor's own 'New theme' button is filed as a Linear
bug against the editor before this ticket lands." The code half was finished, and the gate failed
the ticket twice on that clause, because the only way to answer it was to query Linear, and the
answer was no. A blocking edge would have stopped the build from starting.

### The sequence a bug requires

A bug clause names the sequence of events in the running game that produces the failure. If that
sequence cannot occur, there is no bug.

Measured on GTW-1246. Two stabilize requests were filed and worked as a same-frame double charge,
in a turn-based game where two units cannot submit a request in the same frame. Nobody named the
sequence before the ticket was filed and built.

### Where the expected outcome comes from

A clause that states what a test should assert cites where `docs/` or the ticket says that outcome
is correct. A clause is not evidence for itself.

Measured on GTW-1260. A test asserted that painting outside the grid succeeds, which is the
opposite of the correct behaviour: an out-of-grid paint should return an error. The ticket
inherited the test's premise instead of checking it against the design.

### The symbol a fix changes

A fix clause names the symbol it changes. Coordination added elsewhere — an ordering edge, a
special case, a test-only hook — without touching that symbol is a symptom fix, not the fix.

Measured on GTW-885. The landed fix added ordering-edge tests and drove engine internals through
the MCP. The actual defect was that focus navigation could not reach the new buttons, and the fix
never touched it.

## A sweep ticket's site list comes from its guard

A sweep ticket is one whose work is the same edit at many sites.

Its site list is produced by the command that becomes its conformance guard. Never write the list
by hand. That guard is clause one of the ticket, and the ticket's scope is then whatever the guard
enforces, so a site cannot survive the build.

Read `no_tracked_rust_file_hands_the_frame_clock_back_to_bevy` in
`crates/gdtf_conformance/tests/conformance_suite/no_restored_automatic_time/check.rs` for the shape.
Its site list comes from `offending_lines` in that module's `scan.rs`, which reads every tracked
Rust file. One command gives the ticket its sites and the suite its guard, so the two cannot
disagree.

Measured on GTW-1358. It swept 104 sites for frame and wall-clock budgets from a list written
before the guard existed. The census searched for waits and never searched for handbacks, so seven
tests that pin the clock and hand it back survived the build and became GTW-1365.

## Assertions

An assertion distinguishes zero from many. A helper that returns `Option` for both "none" and
"more than one" cannot carry a clause on its own. Say which count is required, and what the
failure message must tell the reader.

Do not pin a changeable literal. Assert the property instead, per
[verification.md](./verification.md) rule 8.

A predicted line count is a changeable literal, and writing one into a clause turns a guess into
something the build can fail on. Where a size limit matters, name the limit the conformance test
enforces, the block line in `module-layout.md`.

Measured on GTW-1183. Clause 2 said moving two helpers "takes about 30 lines off"
`crates/gdtf_game/tests/game_suite/contextual_panel/emplacement.rs`, and a corrections row asserted a 290-line
count. The file went to 323, because the same clause's mandated signature turned each of eight
call sites from one line into nine. Verify read the gap as a deviation and reddened a run whose
suite was green and whose every clause was met.

## The title

[plain-language.md](./plain-language.md) owns it. Measured on GTW-1251, which was titled "Split
the trapped-emplacement test's control leg into its own test". The user had to ask what it meant.

## Length

A clause is one requirement. If it needs "and also", it is two clauses. A ticket whose clauses
each answer the three questions above needs no prose around them.
