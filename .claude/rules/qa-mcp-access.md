---
paths: ["**/*"]
---

# Driving the running app through the QA MCP

Why this rule exists: an agent that reaches past the QA MCP to a raw socket proves
nothing about the path a real client takes, and hides a broken MCP instead of reporting
it.

## Rules

1. Drive the game and editor only through the `mcp__gdtf-mcp__*` tools: `launch`, `run`,
   `logs`, `stop`, `commands`. That is the whole interface.
2. Never write a socket client. Not a Python script, not `nc`, not a Rust test binary
   that opens the port, not "just to check something". Nothing makes it right, not a
   deadline and not a misbehaving tool.
3. A stale MCP is a build problem, not a routing problem. The MCP host runs the binary
   built from the session's working directory. If the tools do not reflect your change,
   rebuild, then kill the resident process so the next `launch` starts the new one.
4. Still broken after that? Say so and switch tickets. Never gather the evidence another
   way.
5. `design-gate` holds these tools too, and must re-drive live cases rather than trust
   the implementer's transcript. "The gate cannot check this" is false for anything the
   command set can reach.

## Worktrees

`git-workflow.md` bans worktrees partly because of rule 3: work parked in a worktree is
work the running app cannot see. Every clause needing a screenshot or a driven command
depends on the branch being checked out in the main tree.

## Enforcement

Enforced by reading the report. A live-evidence clause is satisfied by a
`mcp__gdtf-mcp__*` transcript and nothing else.
