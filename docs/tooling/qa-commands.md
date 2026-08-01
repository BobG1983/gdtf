---
name: Adding a QA command
description: How to add a command to a QA host — a file, one line in that host's list, and a test — written from the one command that exists, app.phase.
---

# Adding a QA command

A QA command is how an agent drives or reads a running GDTF process. Adding one is **a
file, one line in a host's list, and a test**. It moves no protocol version, adds no wire
variant, and changes nothing in the MCP courier — because a command is DATA carried inside
two frozen envelope variants rather than a variant of its own
([ADR 0008](../decisions/0008-qa-command-courier.md)).

Everything below is written from the ONE command that exists today,
[`crates/gdtf_app/src/dev/net_qa/commands/read/app_phase.rs`](../../crates/gdtf_app/src/dev/net_qa/commands/read/app_phase.rs).
Read that file alongside this one: every shape shown here is in it, at that path. Nothing
here describes a command nobody has written.

## What the QA channel is for

The MCP host exists so an agent can play the game the way a developer would: launch it,
look at it, press the buttons, watch what happens, screenshot it. Same for the editor:
open it, drive the authoring flow, see the result.

It is not a test harness. Anything provable without rendering — sim outcomes, state
transitions, data loading, rule legality — is proven by unit and headless integration
tests, which are cheaper, deterministic, and run in the gate. A QA command that exists to
read internal state so an agent can verify correctness is a test that costs more and
proves less. Write the test instead.

Commands may expose what a player could see or do — which screen is up, what has focus,
what the menu offers — because the agent cannot see pixels cheaply, and orientation is
part of playing. The line: **a command helps an agent play or author, or it does not
exist.** If its only use is asserting something no player could see, it belongs in the
test suite.

Replies stay small and scoped. The old surface's `query_state` answered with the entire
app state — tens of thousands of tokens, mostly irrelevant to the question asked, filling
the caller's context. That is the trap: one convenient dump instead of many scoped reads.
A command answers the one question it names; if a reply could run to pages, the command
is too broad — split it the way a player's perception is split: which screen, what has
focus, what is in view.

## The three edits

1. **A file** under the host's `commands/` directory — one command per file, grouped by
   what it does (`read/` for a command that answers from the world without changing it).
2. **One line** in that host's list. The game's is `GAME_COMMANDS` in
   [`commands/set.rs`](../../crates/gdtf_app/src/dev/net_qa/commands/set.rs); the whole
   entry is `&YourCommand`.
3. **A test.** The suite for the game's command layer is
   [`crates/gdtf_app/tests/net_qa/commands.rs`](../../crates/gdtf_app/tests/net_qa/commands.rs) —
   a real socket, a real listener, and the real router.

## The file

Four items: the argument type, the reply type, the unit struct, and the handler.

### The argument type

```rust
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppPhaseArgs {}
```

`deny_unknown_fields` is not optional. It is what turns "you sent a field I do not have"
into a `BadArguments` answer carrying this type's own derived schema, instead of a silently
ignored key — and it is what puts `"additionalProperties": false` in the schema the
catalogue publishes, so a client can see the strictness before it calls.

`Debug` is required by the trait: a decoded call waits in a `PendingQueue` whose deadline
sweep logs the payload of anything that timed out unclaimed.

### The reply type

```rust
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct AppPhaseReply {
    /// Where the app is, at every level of its state machine.
    phase: AppPhaseNet,
}
```

The reply is where a command's DOMAIN REFUSALS go — "the target is not adjacent", "there is
no selected ganger". `CommandOutcome` has no `Refused` variant on purpose: a command that
runs and says no does so inside its own declared reply type, whose schema the catalogue
publishes.

Anything the reply embeds must derive `JsonSchema` too. The game's five state enums do not,
and deliberately are not made to: `crates/gdtf_app/src/states/` stays free of schema
derives, and
[`commands/../wire/phase.rs`](../../crates/gdtf_app/src/dev/net_qa/wire/phase.rs) mints wire
MIRRORS of them instead, with a wildcard-free `from_state` per level so a new state variant
fails to compile until its mirror gains an arm.

### The command

```rust
impl QaCommand for AppPhase {
    type Args = AppPhaseArgs;
    type Facts = GameFacts;
    type Reply = AppPhaseReply;

    const NAME: CommandName = CommandName::from_static("app.phase");
    const SUMMARY: CommandSummary = CommandSummary::from_static(/* one line */);
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(_facts: &GameFacts) -> CommandAvailability {
        CommandAvailability::Available
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_app_phase.after(QaCommandSystems::Claim));
    }
}
```

- **`Facts`** is the host's, not the command's:
  [`GameFacts`](../../crates/gdtf_app/src/dev/net_qa/facts/game_facts.rs) for the game. It is
  sampled ONCE a frame by the router, so every call in one drain sees the same world. Tying
  it to the host is also what makes a cross-host entry fail to compile at the slice literal.
- **`NAME`** is `family.verb`, lowercase and dotted. It must be unique within the host's
  set; `assert_unique_names` over that set is what proves it.
- **`TIMING`** is a declaration, not a measurement: `Immediate` if the handler answers
  inside `take_calls`, `Deferred` if it parks its responder in `DeferredReplies` and answers
  on a later frame.
- **`availability`** is a PURE function of the frame's facts — no `App`, no queries — so it
  is unit-testable on its own, and the SAME call decides both what the catalogue advertises
  and whether a `Run` is admitted. The two cannot disagree. `app.phase` is always
  `Available` because a command that reports where the app is has to be answerable wherever
  the app is; a command with a precondition returns
  `CommandAvailability::Unavailable { code, note }`, naming the missing thing in words a
  caller can act on.
- **`register_handler`** puts the handler in whatever schedule band its work belongs to. The
  only requirement is `.after(QaCommandSystems::Claim)` — that is where the decode step fills
  the queue, so a handler that ran earlier would answer every call a frame late.

### The handler

```rust
fn handle_app_phase(
    facts: GameFactsParam,
    mut queue: ResMut<PendingQueue<CommandCall<AppPhase>>>,
) {
    if queue.is_empty() {
        return;
    }
    let sampled = facts.sample();
    for (_args, responder) in take_calls::<AppPhase>(&mut queue) {
        responder.answer(&AppPhaseReply {
            phase: sampled.phase(),
        });
    }
}
```

An ordinary Bevy system with ordinary system params. It never sees JSON and never sees a raw
`Responder`: it gets `C::Args` in and answers `&C::Reply` out, so the shape a client was
promised in the catalogue is the only shape it can produce.

The `is_empty` early-out is not a micro-optimisation. `take_calls` takes `&mut`, which
dirties the queue's change-detection flag, and on a host with many commands almost every
frame has nothing to drain.

## What you do NOT edit

- **The protocol.** No `QaRequest` variant, no `QaResponse` variant, no version bump.
- **The router.** `Catalogue` and `Run` are already routed. In particular, do NOT add a
  second system that reads `Res<NetInbox>`: `drain()` takes everything in the channel, so
  two readers means whichever runs first swallows the other's requests. The property is
  pinned by
  [`crates/gdtf_app/tests/net_qa/command_set.rs`](../../crates/gdtf_app/tests/net_qa/command_set.rs).
- **The MCP courier.** `commands` and `run` carry any command by name. Neither names one,
  and neither should learn to.

## Testing it

Two layers, both cheap:

- **The set.** `assert_game_command_set_is_conformant()` runs both per-host assertions —
  unique names, parseable schemas — over the real slice. It is already registered; a new
  command is covered by it the moment it joins the list.
- **The command.** Add a case to
  [`crates/gdtf_app/tests/net_qa/commands.rs`](../../crates/gdtf_app/tests/net_qa/commands.rs).
  Its `exchange` helper negotiates and sends over a real socket into the real router, so a
  case there exercises the whole path a live client drives. Assert on the reply's JSON rather
  than on derived schema TEXT: pinning schema text breaks on any `schemars` release without a
  behaviour changing.

## Calling it

From an MCP client, two tools and no more:

```text
commands(host="game")                                    # what can I call?
commands(host="game", command="app.phase", detail="Full") # what does it take?
run(host="game", command="app.phase", arguments={})       # do it
```

`commands` reads the catalogue from the RUNNING host, so its `availability` column is the
live answer rather than a static claim. Four things can go wrong, and each tells you how to
fix it in one round trip: `Unknown` lists every name the host does offer, `BadArguments`
carries the schema your body failed against, `Unavailable` names the precondition that is
missing, and a rider this build has not implemented (`await_ready`, `capture`) is refused
`Unavailable` with code `NotBuilt` rather than run without it.
