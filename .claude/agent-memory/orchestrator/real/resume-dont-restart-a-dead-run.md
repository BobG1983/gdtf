---
name: resume-dont-restart-a-dead-run
description: A workflow that dies on an overload or a session limit lands nothing — resume from the run id and rewrite only the failed agent's prompt; keep the script at a repo path; never poll a sentinel.
metadata:
  type: feedback
---

An agent dying on `529 Overloaded` or a session limit means nothing landed and nothing is wrong
with the work. **Resume from the run id** — cached phases replay free. Then read the partial work
actually on disk and rewrite **only** the failed agent's prompt to match what is left.

**Why:** a shorter prompt means less overload exposure on the retry, and re-running the whole
thing invites the same death. Restarting from scratch also discards phases that already
succeeded, which is the expensive half of the run.

**How to apply:**

- **Keep the workflow script at a repo path.** `.claude/workflows/build-ticket.js` — a script
  written into the session scratchpad dies with the session, and the resume then fails with
  "adopt scriptPath rejected", leaving the run checkpointed and unrecoverable. Resuming with a
  *different* scriptPath is fine: the agent cache is keyed by prompt and options, not by file
  location, so a script can be moved mid-run and completed phases still replay.
- **A workflow cannot resume another workflow.** Resume from the main session. `/heartbeat`
  follows this, and `.claude/run-state.md` carries the in-flight branch and run id
  for exactly that recovery.
- **Read the marker file first in any fresh session.** `.claude/run-state.md` is what
  stops a double-start and two concurrent cargo builds.
- **Compaction does not kill a run; a process restart does.** A compact makes you forget the task
  id — wait for the notification. A model change or crash kills it, and the task lookup then
  returns "no task found": relaunch or resume.
- **Judge a death from the last records, never from mtime.** A recent transcript mtime proves a
  recent write — an interruption writes on its way out. A bare `started` with no `result`, or an
  interruption marker, means dead. Zero tokens and zero tool calls means the agent never spawned
  and nothing was lost; real tokens then death means read the worktree before assuming anything. A
  repeated cache key proves a retry, never a cause — read the last tool call of every agent in the
  run before editing the script.
- **Never poll a sentinel.** Do not background a cargo command or build a sleep-until loop around
  a marker file; a silent death becomes a deadlock with no signal. Run one foreground command per
  call. A multi-hour external command cannot be babysat by a subagent — there is no wake-up
  mid-call. Hand it to the user's terminal, peek at its output file without blocking, and triage
  the result as a separate short task.
- **Do not spin.** Repeating an identical no-op status check turn after turn is not progress. Do
  collision-safe work instead, hand back with a recommendation, or schedule one longer wakeup. A
  build lock forbids cargo; it does not forbid diagnosis, edits, or filing tickets. Back off in
  minutes, not seconds.

Related: [[build-it-in-a-workflow]], [[a-dead-agent-is-not-a-pass]].
