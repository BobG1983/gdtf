# Background work: never poll, always relay

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

Start everything in the background, keep working, and report every result that
lands.

## Rules

1. Never poll for work the harness tracks. A background job or sub-agent
   re-invokes the main session when it finishes. A loop that sleeps and checks
   burns turns to learn what the notification will say. That includes "just one
   quick check on progress".
2. To wait on something the harness cannot see, such as an external process or a
   file another machine writes, use `Monitor`, or one background command that
   exits when its condition is true. Never take repeated turns that check.
3. Always run sub-agents in the background. Every one, including the one whose
   answer you want most. A foreground agent blocks the session while it runs.

   One exception: an agent that needs the `LSP` runs in the foreground. A
   backgrounded sub-agent never gets it, whatever its definition says, and
   `ToolSearch` cannot fetch what was never granted. Both were measured.
   Workflow agents are not background sub-agents and keep the `LSP`.
4. Relay every sub-agent report. A result folded silently into other work has
   not been relayed.
5. Never state a pending agent's result. Until the notification arrives, the
   only true thing to say is that it is still running.
6. Never idle on a blocking question. Put it on the ticket as a comment per
   [linear-discipline.md](./linear-discipline.md), apply `Needs User Input`, and
   leave the status at Backlog. Then pick up unblocked work.
7. Never call `AskUserQuestion` unless the user asked for it. It stops the whole
   harness until someone answers, and the user is often away.

   Measured 2026-08-21: three build workflows sat unstarted for between ninety
   minutes and two and a half hours each, and every one was behind an
   `AskUserQuestion` nobody had asked for. A tick blamed the project-manager
   agents before the user named the real cause.

   A question the user has not asked for goes on the ticket, under rule 6.

## Inside a sub-agent, run everything in the foreground

Ending your turn ends the agent, and a backgrounded shell command it left
running dies unread. Run each command in the foreground and read its exit code
in the same turn. Several commands, one at a time.

A land agent once started the green suite with `run_in_background` and ended its
turn saying a monitor would report the exit codes. Nothing was monitoring it.
Every other phase had passed, and the work sat uncommitted.

Whether a backgrounded child agent's completion reaches its sub-agent parent has
not been measured here. Pass `run_in_background: false` and the question does not
arise.

## What goes in a sub-agent prompt

Carry what the agent cannot find for itself: the user's ruling, the citation, the
quoted output, the decision already taken. Nothing else.

The agent writes its ticket or report in the style of your prompt, so a long
chatty prompt produces a long chatty ticket. Measured: a run of tickets the user
found verbose each came from a prompt several times longer than the ticket
needed.

Before sending, ask whether each sentence would survive into the ticket or the
report. If not, cut it. That cuts narration of what you already did, your
reasoning, facts the agent can read in the tree, and anything about how the
prompt is written.

[reply-shape.md](./reply-shape.md) owns the shape of a relay.
