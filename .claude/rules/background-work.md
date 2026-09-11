# Background work: never poll, always relay

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

Start everything in the background unless the background agent needs `LSP` access,
 keep working, and report every result that lands.

## Rules

1. Never poll for work the harness tracks. A background job or sub-agent
   notifies the main session when it finishes.
   That includes "just one quick check on progress".
2. To wait on something the harness cannot see, such as an external process or a
   file another machine writes, use `Monitor`. Never take repeated turns that check.
3. Always run sub-agents in the background. Every one, including the one whose
   answer you want most.

   **One exception:** an agent that needs the `LSP` runs in the foreground. A
   backgrounded sub-agent never gets it, whatever its definition says, and
   `ToolSearch` cannot fetch what was never granted.
4. Relay every sub-agent report. A result folded silently into other work has
   not been relayed.
5. Never state a pending agent's result. Until the notification arrives, the
   only true thing to say is that it is still running.
6. Never idle on a blocking question. Use `AskUserQuestion`, it has a 10-minute timeout.
   If the user does not answer, the harness will report it, and you should continue with the next work item. Do not wait for a user answer that may never come.
7. If the user does not answer a question from `AskUserQuestion`, and you have continued to work
   on other items, let the user know when they return that the question was not answered and ask
   it again now they are present.

## Sub-agents **MUST** run everything in the foreground

If you are a sub-agent (not the Orchestrator), run each command in the foreground
and read its exit code in the same turn. Several commands, one at a time.

## What goes in a sub-agent prompt

Carry what the agent cannot find for itself: the user's rulings, citations,
quoted output, decisions already made, tests already tested, findings already found.

The agent writes its ticket or report in the style of your prompt. **BE CONCISE**

Before sending, ask whether each sentence should survive into the ticket or the
report. If not, cut it.

[reply-shape.md](./reply-shape.md) owns the shape of a reply to the user.
