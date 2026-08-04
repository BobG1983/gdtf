---
name: pattern-unsatisfiable-acceptance-clause
description: Some clauses demand live evidence no permitted tool can produce — settle it in code before ruling either way, and never bridge the gap with a script.
metadata:
  type: feedback
---

When a clause demands live evidence from a running process, confirm in CODE what the permitted
tools can actually reach before ruling. "The tool does not exist" is a hypothesis, not a
finding, and so is "it does exist".

**Why:** the answer moves. The MCP bridge once ran only the game; today
`bins/gdtf_qa_mcp/src/lifecycle/spawn.rs:38-54` builds `cargo run -p …` from
`spec.package()`, and `LaunchSpec::editor_default()` names `gdtf_content_editor_bin`
(`lifecycle/launch/spec.rs:12`), so `launch(host="editor")` starts the editor —
`spawn.rs:139` `editor_recipe_runs_the_editor_binary` pins it. `QaHost` has both hosts
(`hosts/host.rs:16-21`) and picks the recipe per host (`:71-76`). What a host offers is no
longer in the tool list at all: there are five tools (`mcp/tools/name.rs:5-16`,
`ALL` at `:18-24`), and the per-host command catalogue is data the running child answers with,
via `commands` and `run`. Ruling from a remembered tool list is how a satisfiable clause gets
called impossible.

**How to apply:** (1) list the tools actually exposed to you; (2) read `ToolName` and the
spawner for the target binary; (3) if the evidence really is unreachable, rule VIOLATION with
those file:lines and say what would unblock it — not "blocked". A pre-authorized deferral
written inside a ticket description is not a satisfied clause. Never bridge the gap with a
script or a socket client; the correct output is a cited, unmet clause.

Related: [[pattern-untested-mcp-description-prose]].
