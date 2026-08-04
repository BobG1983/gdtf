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

`LSP` is loaded deferred — load it once with `ToolSearch` before the first call.

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

## The tool reads. It does not edit

Those eight plus `prepareCallHierarchy` are the whole set — nine operations, none of which
edit. No rename, no code action, no quick-fix, no diagnostics.

**For navigation and investigation, ALWAYS use the `LSP` tool, never the `rust-analyzer`
binary.** 

For editting, use the `rust-analyzer` binary. The binary is for the things the tool cannot do, for example:

```bash
rust-analyzer ssr '$a.foo($b) ==>> bar($a, $b)'   # structural search and replace
rust-analyzer search '$a.foo($b)'                 # the same matching, no rewrite
```

`ssr` is type- and syntax-aware across the workspace — the right tool for "change this call
shape everywhere", and far better than fifty hand edits or a `sed` that cannot tell a call
from a comment. It rewrites files in place, so run it on a branch, keep the suite green
either side, and read the diff.

Also there: `diagnostics`, `unresolved-references`, `analysis-stats`, `lsif`/`scip` index
dumps, and likely more.

**Check the binary's `--help` rather than trusting memory or this file.** rust-analyzer's
own help says its subcommands "do not provide any stability guarantees and may be removed
or changed without notice".

## How it is enforced

When you are about to write a number into a ticket, a report, or a commit message:
*did something that understands Rust produce this, or did a text search?* If a text
search, go and get the real one.
