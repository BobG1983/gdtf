# Clause writing: a clause a builder can satisfy and a run can prove

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

This file applies to anyone writing ticket text: `/file-bug`, `/next-task`, and the `project-manager`
agent. `clause-audit` checks against it.

[linear-discipline.md](./linear-discipline.md) owns how a clause cites things: the symbol or the
quoted text, never a bare line number. This file owns whether the clause can be built and shown.

## Every clause answers three questions

1. What changes? Name the behaviour concretely.
2. Where? Name the symbol and the file that holds it, and check it exists right now.
3. What goes red if it is wrong? Name the test, and the mutation it catches.

A clause missing the third is a wish. "No longer depends on the default order" is true the moment
the ordering edge lands, so nothing can fail. Rewrite it as the case a test can run and fail.

## The nine things a clause pins down

### Direction, for anything ordered

An ordering clause names both endpoints in order: A before B, not "an edge between them". The two
directions are different tickets, and the wrong one can void another clause.

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

### The sequence a bug requires

A bug clause names the sequence of events in the running game that produces the failure. If that
sequence cannot occur, there is no bug.

### Where the expected outcome comes from

A clause that states what a test should assert cites where `docs/` or the ticket says that outcome
is correct. A clause is not evidence for itself. Check an existing test's premise against the
design before a clause inherits it.

### The symbol a fix changes

A fix clause names the symbol it changes. An ordering edge, a special case, or a test-only hook
added elsewhere, without touching that symbol, is a symptom fix.

## Finding every place that needs the same edit

Some tickets make the same edit in many places. Find those places with a search, and put the search
in the ticket so a reader can run it again. Never type the list out yourself, because you will miss
some.

Check the search both ways before the list goes in. Does it find everything the edit changes? Does
everything it finds need the edit?

What goes red is a unit test or an integration test, the same as for any other clause. Conformance
guards pin repo structure and are rare, so this kind of ticket does not get one. Where no test can
catch a place that comes back later, the clause names the search, says it must return nothing, and
the gate runs it on the final tree.

## Assertions

An assertion distinguishes zero from many. A helper that returns `Option` for both "none" and
"more than one" cannot carry a clause on its own. Say which count is required, and what the
failure message must tell the reader.

Do not pin a changeable literal. Assert the property instead, per
[verification.md](./verification.md) rule 8.

A predicted line count is a changeable literal, and writing one into a clause turns a guess into
something the build can fail on. Where a size limit matters, name the limit the conformance test
enforces, the block line in `module-layout.md`.

## The title

[plain-language.md](./plain-language.md) owns it.

## Length

A clause is one requirement. If it needs "and also", it is two clauses. A ticket whose clauses
each answer the three questions above needs no prose around them.
