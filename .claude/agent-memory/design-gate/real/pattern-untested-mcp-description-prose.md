---
name: pattern-untested-mcp-description-prose
description: The MCP tool descriptions are the text clients read to decide how to call a tool, and nothing asserts a word of them — deleting a qualifier fails nothing.
metadata:
  type: feedback
---

Tool description prose in `bins/gdtf_qa_mcp` has no test backing it. When a clause changes
that prose, name the mutation it leaves uncovered.

**Why:** the descriptions live in one private match,
`bins/gdtf_qa_mcp/src/mcp/tools/describe.rs:4-51`, and reach clients through
`ToolName::descriptor()` (`tools/schema.rs:119-125`) and `tools_list_result()` (`:127-132`).
They carry real instructions a caller acts on — which values `host` takes, that `commands`
must be called before `run`, that `capture` accepts `true` or a file-stem string, that a child
from a different recipe is refused. The only test over `tools/list`,
`tests/jsonrpc/protocol.rs:18-29`, checks the five NAMES and the array length; it never looks
at a description. Delete a qualifier from any of those strings and the whole suite stays
green.

Two traps when checking this. `bins/gdtf_qa_mcp/src/mcp/tools/test/` looks like the missing
coverage — `test/courier.rs:167` even asserts what no description may name — but
`tools/mod.rs:1-8` declares `describe`, `name` and `schema` and no `test` module, so nothing
in that directory is compiled. And the schemas in `tools/schema.rs` carry their own per-field
`description` strings (`:17`, `:27-43`, `:62`, `:74-100`) with the same absence of coverage.

**How to apply:** treat a description change as an uncovered gap and say which sentence a
future edit could silently drop. Do NOT demand an exact-string pin — brittle, and it makes
every wording fix a test edit. A `contains` check on the load-bearing word, read through
`tools_list_result()` so it goes through the real path, is the proportionate ask.

Related: [[pattern-test-doc-overclaims-its-assertion]], [[pattern-sweep-deletes-a-policy-section]].
