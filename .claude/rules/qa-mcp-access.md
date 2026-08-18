---
paths: ["**/*"]
---

# Driving the running app — the MCP tools, and nothing else

Why this rule exists: the QA MCP is not a convenience wrapper around a socket. It 
is **the** way the game and editor get looked at by agents. 
An agent that reaches past it to a raw socket proves nothing about
the path a real client takes, and hides a broken MCP instead of reporting it.

## Rules

1. **Drive the game and editor only through the `mcp__gdtf-qa__*` tools.** 
   `launch`, `run`, `logs`,`stop`, `commands`. That is the whole interface.
2. **Never write a socket client.** Not a Python script, not `nc`, not a Rust test binary
   that opens the port, not "just to check something". There is no case where this is the
   right move, including a deadline and including a tool that is misbehaving.
3. **A stale MCP is a build problem, not a routing problem.** The bridge runs the binary
   built from the session's working directory. If the tools do not reflect your change:
   rebuild, then kill the resident process so the next `launch` starts the new one.
4. **Still broken after that? Say so and switch tickets.** A blocked live-evidence clause
   is a real blocker to report, never a licence to gather the evidence another way.
5. **`design-gate` holds these tools too**, and is expected to re-drive live cases rather
   than take the implementer's transcript on trust. "The gate cannot check this" is false
   for anything the command set can reach.

## Why worktrees interact with this

`git-workflow.md` bans worktrees partly for this reason: the host builds from the
session's working directory, so work parked in a worktree is work the running app cannot
see. Every clause needing a screenshot or a driven command depends on the branch being
checked out in the main tree.

## How it is enforced

By reading the report. A live-evidence clause is satisfied by a `mcp__gdtf-qa__*`
transcript and nothing else. Evidence from any other transport is not weaker evidence —
it is not evidence, because it did not exercise the thing being built.
