---
paths: ["**/*.rs"]
---

# Ask the LSP about symbols, not grep

Why this rule exists: a ticket was sized from `grep -c` — "`march_vector` has 41 callers".
The LSP said 52 references across 20 files. Every function re-checked was undercounted the
same way, `reachable_within` by half. The numbers were not low, they were a different
quantity: **`grep -c` counts lines containing a string.** It has no idea what a caller is.
Those numbers reached a builder as the size of their work.

`rust-analyzer` serves every `.rs` file in this repo. Use it.

## The rule

1. **Symbol question → LSP.** Who calls this, where is it defined, what type is it, what
   does this file export, what implements this trait.
2. **Never state a count of anything from `grep`.** Not callers, not references, not call
   sites. If you write a number, name the operation that produced it.
3. **Run `findReferences` before changing a signature.** That list is the work. Read all of
   it first — after the first edit the old symbol is gone and the list cannot be re-derived.
4. **`grep` is for text that is not a symbol:** a phrase in a doc, a `reason =` string, a
   filename, a lint name, a config key. Reach for it on purpose, not by reflex.

`LSP` is loaded deferred — load it once with `ToolSearch` at the START of a run, not when you
first need it. rust-analyzer starts on that first call and takes a few minutes to index this
workspace, so ask early and it is warm when it matters. If an early call says indexing has
not finished, that means cold, not broken — retry. (`cargo build` does not help; rust-analyzer
keeps its own index.)

**A cold answer is EMPTY, not an error — and empty looks exactly like a real zero.** Measured
2026-08-04: `workspaceSymbol` for `apply_suppression`, a symbol that plainly exists, returned in
5 seconds with *"No symbols found in workspace. This may occur if the workspace is empty, or if
the LSP server has not finished indexing the project."* No error, no hang.

File-local operations (`documentSymbol`, `hover`, `goToDefinition`) answer while cold.
Workspace-wide ones (`findReferences`, `workspaceSymbol`, `goToImplementation`, the call
hierarchy) need the full index and will quietly under-report until it is built.

## Which operation

| Question | Operation |
| --- | --- |
| Who mentions this? | `findReferences` — includes imports and the definition |
| Who actually **calls** this? | `incomingCalls` — the call graph, nothing else |
| What does this call? | `outgoingCalls` |
| Where is it defined? | `goToDefinition` |
| What type is this, really? | `hover` |
| What does this file define? | `documentSymbol` |
| Where is this symbol at all? | `workspaceSymbol` (always pass a query) |
| What implements this trait? | `goToImplementation` |

`incomingCalls` and `outgoingCalls` want a `prepareCallHierarchy` at the same position
first.

**Three different quantities, and they are not interchangeable:** a grep count, a
`findReferences` count, and an `incomingCalls` count. Conflating them is the whole defect
this rule exists to stop.

## Four of these enforce rules we otherwise check by eye

- **`goToImplementation` finds the house bundles.** [`bevy-systems.md`](./bevy-systems.md)
  says mirror an existing `SystemParam` instead of inventing a shape. This gives the real
  list; grepping `derive(SystemParam)` gives only the ones written that way.
- **`documentSymbol` proves a `mod.rs` is wiring-only.**
  [`module-layout.md`](./module-layout.md) rule 2 forbids any `fn` there. This lists what a
  file defines; grepping `fn` hits comments, doc examples and strings.
- **`outgoingCalls` finds the decomposition boundary.** When a function trips
  `too_many_lines`, what it calls is where it splits.
- **`hover` says whether a value is really a newtype.**
  [`no-bare-types.md`](./no-bare-types.md) bans bare domain types, and a parameter named
  `hp` tells you nothing about whether it is an `Hp` or a `u32`.

## Two tools, both encouraged, different jobs

Those eight plus `prepareCallHierarchy` are the whole `LSP` tool — nine operations, all
read-only. No rename, no code action, no quick-fix, no diagnostics.

**Navigation and investigation → the `LSP` tool.** Who calls this, what type is this, what
does this file define. Do not shell out to the binary to answer a question the tool answers.

**Bulk edits → the `rust-analyzer` binary, through `Bash`. Use it freely — it is the
preferred way to make the same change at many call sites.**

```bash
rust-analyzer ssr '$a.foo($b) ==>> bar($a, $b)'   # structural search and replace
rust-analyzer search '$a.foo($b)'                 # the same matching, no rewrite
```

`ssr` is type- and syntax-aware across the whole workspace. It knows a call from a comment,
a doc example, or a string literal — which is exactly what every text-based approach gets
wrong. It rewrites in place, so work on a branch, keep the suite green either side, and read
the diff.

Also there: `diagnostics`, `unresolved-references`, `analysis-stats`, `lsif`/`scip` index
dumps, and likely more.

**Check the binary's `--help` rather than trusting memory or this file.** rust-analyzer's
own help says its subcommands "do not provide any stability guarantees and may be removed
or changed without notice".

## Do not write a script to edit source

Why this rule exists: an engineer migrating call sites across ~90 files wrote a Python
rewriter in the scratchpad, then spent most of the run patching what it did — including one
pass that walked every `.rs` file in the repo injecting `use` lines, and three more passes
removing the ones it got wrong.

**Banned: ad-hoc Python, `sed`, `awk` or `perl` that edits Rust source.** Two reasons, and
the second is the one that decides it:

- Regex cannot tell a call from a comment, a doc example, or a string literal.
- The script has to be written, run, debugged and rewritten. That costs more than the edits
  it was meant to save — measured, not assumed.

The order to reach for:

1. **`rust-analyzer ssr`** — the same change at many sites. This is the tool for the job.
2. **`Edit`** — anything ssr cannot match. Yes, one at a time. It is faster than it looks,
   and every change is visible in the diff.
3. **Say so** — if a change fits neither, that is a signal the ticket needs splitting, not
   that it needs a script.

Scripts that *read* are fine: counting, listing, searching for a `reason` string. The ban is
on generating source edits. If a rewrite genuinely cannot be expressed in ssr, report that
and say why — a real finding, not a licence to write the script anyway.

## How it is enforced

When you are about to write a number into a ticket, a report, or a commit message:
*did something that understands Rust produce this, or did a text search?* If a text
search, go and get the real one.
