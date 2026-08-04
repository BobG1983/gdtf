---
paths: ["**/*"]
---

# Background work — never poll, always relay

Why this rule exists: on 2026-08-04 a long stretch of the session landed nothing.
Three causes, all of them waiting dressed up as work. The session ran shell loops
to watch jobs that already announce their own completion. It ran sub-agents in
the foreground, which blocks everything until they finish. And it put four
decisions to the user in chat and then had nothing to do, because the questions
were not on their tickets and no unblocked work was picked up.

Backgrounding was never the problem — not relaying the result is. Spawn
everything in the background and keep working; the failure to guard against is a
result landing and never reaching the user.

## Rules

1. **Never poll for work the harness tracks.** A background job or sub-agent
   re-invokes the session when it finishes. A loop that sleeps and checks on it
   burns turns to learn what the notification was going to say anyway. This
   covers "just one quick check on how far along it is".
2. **To wait on something the harness cannot see** — an external process, a file
   another machine writes — use the `Monitor` tool, or one background command
   that exits when its condition becomes true. One command that waits, never
   repeated turns that check.
3. **Always run sub-agents in the background.** Every one, including the one
   whose answer you want most. A synchronous agent blocks the session for as
   long as it runs, which is the same idling this rule exists to stop. Spawn it,
   carry on with something else, and relay the result when it lands.
4. **Relay every sub-agent report.** The agent reports to the session; the
   session reports to the user. A result that lands and gets folded silently
   into other work has not been relayed.
5. **Never state or predict a pending agent's result.** Until the completion
   notification arrives, the only true thing to say is that it is still running.
6. **Never idle on a blocking question.** The question goes on its ticket per
   [linear-discipline.md](./linear-discipline.md) — comment, `Needs User Input`,
   status stays Backlog — and then you pick up unblocked work. Waiting is not
   progress, and an answer that lives only in chat does not survive the session.

## How it is enforced

Rule 3 has nothing to decide — every sub-agent is backgrounded, so there is no
judgment call to get wrong. The other five are checked by asking:

- Rules 1 and 2, before writing a loop that sleeps: *will something notify me?*
  If yes, do not write it.
- Rule 6, after spawning anything: *what am I doing while that runs?* "Waiting"
  is the wrong answer. There is always either other work or a reply to write.
- Rule 5, before describing anything in flight: *has its notification arrived?*
  If no, the only true sentence is that it is still running.
- Rule 4, when a notification lands: *has the user been told?* Relay it before it
  gets folded into something else.

[reply-shape.md](./reply-shape.md) owns the shape of that relay. This file owns
whether it happens at all.
