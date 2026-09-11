---
paths: ["**/*.rs"]
---

# Ask the LSP about symbols, not grep

`rust-analyzer` serves every `.rs` file in this repo via `LSP` for read-only access,
and via `Bash` for write operations. Use it.

## The rule

1. Ask the LSP about a symbol: who calls this, where is it defined, what type is it, what
   does this file export, what implements this trait.
2. Never state a count of anything from `grep`. If you write a number, name the operation
   that produced it. If a text search produced it, go and get the real one.
3. Run `findReferences` before changing a signature. That list is the work. Read all of it
   first. After the first edit the old symbol is gone and the list cannot be re-derived.
4. Use `grep` for text that is not a symbol: a phrase in a doc, a `reason =` string, a
   filename, a lint name, a config key. Reach for it on purpose, not by reflex.

## Loading the LSP tool

`LSP` is deferred, so load it with `ToolSearch` at the START of a run. rust-analyzer starts
on that first call and takes a few minutes to index this workspace. An early call that says
indexing has not finished means cold, not broken, so retry. `cargo build` does not help,
because rust-analyzer keeps its own index.

A cold answer is EMPTY, not an error, and empty looks exactly like a real zero. A cold
`workspaceSymbol` returns *"No symbols found in workspace. This may occur if the workspace
is empty, or if the LSP server has not finished indexing the project."*

File-local operations (`documentSymbol`, `hover`, `goToDefinition`) answer while cold.
Workspace-wide ones (`findReferences`, `workspaceSymbol`, `goToImplementation`, the call
hierarchy) need the full index and will quietly under-report until it is built.

## Which operation

| Question | Operation |
| --- | --- |
| Who mentions this? | `findReferences` (includes imports and the definition) |
| Who actually **calls** this? | `incomingCalls` (the call graph, nothing else) |
| What does this call? | `outgoingCalls` |
| Where is it defined? | `goToDefinition` |
| What type is this, really? | `hover` |
| What does this file define? | `documentSymbol` |
| Where is this symbol at all? | `workspaceSymbol` (always pass a query) |
| What implements this trait? | `goToImplementation` |

`incomingCalls` and `outgoingCalls` want a `prepareCallHierarchy` at the same position
first.

A grep count, a `findReferences` count and an `incomingCalls` count are three different
quantities.

## Operations that enforce other rules

`goToImplementation` on `SystemParam` lists every one the repo already has.
[`bevy-systems.md`](./bevy-systems.md) says mirror an existing `SystemParam` instead of
inventing a shape, and grepping `derive(SystemParam)` gives only the ones written that way.

`documentSymbol` proves a `mod.rs` is wiring-only. [`module-layout.md`](./module-layout.md)
rule 2 forbids any `fn` there. Grepping `fn` hits comments, doc examples and strings.

When a function trips `too_many_lines`, `outgoingCalls` shows what it calls, and that is
where it splits.

`hover` says whether a value is a newtype. [`no-bare-types.md`](./no-bare-types.md)
bans bare domain types, and a parameter named `hp` tells you nothing about whether it is an
`Hp` or a `u32`.

## The LSP tool and the rust-analyzer binary

Those eight operations plus `prepareCallHierarchy` are the whole `LSP` tool: nine, all
read-only, and no diagnostics.

Do not shell out to the binary to answer a question the `LSP` tool answers.

Bulk edits go to the `rust-analyzer` binary, through `Bash`.

```bash
rust-analyzer ssr '$a.foo($b) ==>> bar($a, $b)'   # structural search and replace
rust-analyzer search '$a.foo($b)'                 # the same matching, no rewrite
```

`ssr` is type- and syntax-aware across the whole workspace. It rewrites in place, so work
on a branch, keep the suite green either side, and read the diff.

The binary also carries `diagnostics`, `unresolved-references`, `analysis-stats`,
`lsif`/`scip` index dumps, and likely more.

Check the binary's `--help` rather than trusting memory or this file. rust-analyzer's own
help says its subcommands "do not provide any stability guarantees and may be removed or
changed without notice".

## Do not write a script to edit source

Ad-hoc Python, `sed`, `awk` or `perl` that edits Rust source is banned. Regex cannot tell a
call from a comment, a doc example, or a string literal. The script also has to be written,
run, debugged and rewritten, which costs more than the edits it saves.

The order to reach for:

1. `rust-analyzer ssr`, for the same change at many sites.
2. `Edit`, for anything ssr cannot match. One at a time is faster than it looks, and
   every change is visible in the diff.
3. Say so, and say why. A change that fits neither is a signal the ticket needs splitting,
   not that it needs a script.

Scripts that *read* are fine: counting, listing, searching for a `reason` string.
