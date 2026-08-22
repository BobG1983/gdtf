# Clause writing — a clause a builder can satisfy and a run can prove

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything
> here. It is binding on every word, and it is not optional.**

Why this rule exists: the clause audit keeps correcting the same five things, and every one of
them was decided when the ticket was written. The audit knew what a bad clause looked like; the
writer had no list. This is that list.

It applies to anyone writing ticket text — `/file-bug`, `/next-task`, the `project-manager`
agent — and `clause-audit` checks against it, so the writer and the auditor use one list.

[linear-discipline.md](./linear-discipline.md) owns how a clause cites things: the symbol or the
quoted text, never a bare line number. This file owns whether the clause can be built and shown.

## Every clause answers three questions

1. **What changes** — the behaviour, named concretely.
2. **Where** — the symbol and the file that holds it, verified to exist right now.
3. **What goes red if it is wrong** — the test, and the mutation it catches.

A clause missing the third is a wish. "No longer depends on the default order" is true the moment
the ordering edge lands, so nothing can fail; rewrite it as the case that reads the downed
selection and the update count that keeps it readable.

## The six

**Direction, for anything ordered.** An ordering clause names both endpoints in order — A before
B, not "an edge between them". The two directions are different tickets, and the wrong one can
void another clause: on GTW-1144, putting the clear first would have moved the selection before
the offer systems read it, which is the exact case three other clauses existed to pin.

**The owner of the edge.** Say which crate configures it. A set defined in one crate cannot be
named by a crate that does not depend on it, so "add the edge" has exactly one legal home and the
clause should say so.

**The helper's behaviour, when evidence leans on it.** If the assertion goes through a test helper,
state the part of its behaviour the clause depends on. `ordered_before` walks direct edges only,
not transitively, so an edge attached per system instead of on the set leaves the assertion false
while the edge is present — and the builder cannot see why from the clause.

**Only things that exist.** Do not name a per-item spawn when registration is generic. Open the
file and read it before writing the clause; the audit will, and finding it there is cheaper.

**A mutation that fails for the stated reason.** Naming a mutation is evidence only if the red run
is caused by the thing the clause is about. A second wielded weapon that flips an earlier assertion
proves nothing about the guard. Say which assertion must go red, and rule out the other ways it
could.

**Nothing a clause asks for lives off the tree.** **A clause never says "file a bug."** Nor "the
ticket exists", nor "the label is off", nor anything else answered by reading the board. A clause
is satisfied by code, and shown by a run. Work that has to happen first but leaves no trace in the
tree is a **precondition**: file it, and make it a blocking edge on the ticket.

Measured on GTW-1002, whose clause 7 read "The editor's own 'New theme' button is filed as a Linear
bug against the editor before this ticket lands." Its code half was finished and the gate failed
the ticket twice on that clause, because the only way to answer it was to query Linear — and the
answer was no. A blocking edge would have stopped the build from starting, which is what a
precondition is for. Instead it burned two gate rounds on a ticket that could not pass.

## Assertions

An assertion distinguishes zero from many. A helper returning `Option` for both "none" and "more
than one" cannot carry a clause on its own — say which count is required and what the failure
message must tell the reader.

Do not pin a changeable literal: assert the property, per
[verification.md](./verification.md) rule 8.

A predicted line count is a changeable literal, and writing one into a clause turns a guess into
something the build can fail on. Where a size limit matters, name the limit the guard enforces —
`module-layout.md`'s block line — never a count the change is expected to land on.

Measured on GTW-1183. Clause 2 said moving two helpers "takes about 30 lines off"
`crates/gdtf_app/tests/contextual_panel/emplacement.rs`, and a corrections row asserted a 290-line
count. The file went to 323, because the same clause's mandated signature turned each of eight call
sites in that file from one line into nine. Verify read the gap as a deviation and reddened a run
whose suite was green and whose every clause was met.

## The title

[plain-language.md](./plain-language.md) owns it. Measured here: GTW-1251 was titled "Split the
trapped-emplacement test's control leg into its own test", and the user had to ask what it meant.

## Length

A clause is one requirement. If it needs "and also", it is two clauses. A ticket whose clauses
each answer the three questions above needs no prose around them.
