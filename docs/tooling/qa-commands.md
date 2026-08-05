---
name: Adding a QA command
description: How to add a command to a QA host — a file, one line in that host's list, and a test — written from app.phase, the simplest command the game publishes.
---

# Adding a QA command

A QA command is how an agent drives or reads a running GDTF process. Adding one is **a
file, one line in a host's list, and a test**. It moves no protocol version, adds no wire
variant, and changes nothing in the MCP courier — because a command is DATA carried inside
two frozen envelope variants rather than a variant of its own
(see [Why the shape is what it is](#why-the-shape-is-what-it-is) below).

Everything below is written from the simplest command the game publishes,
[`crates/gdtf_app/src/dev/net_qa/commands/read/app_phase.rs`](../../crates/gdtf_app/src/dev/net_qa/commands/read/app_phase.rs).
Read that file alongside this one: every shape shown here is in it, at that path. Nothing
here describes a command nobody has written. The game's other reads — `settings.read`,
`ui.focus` and `playback.state`, beside it under `commands/read/` — are the same four items
reading a different resource, and
[`commands/capture/screenshot.rs`](../../crates/gdtf_app/src/dev/net_qa/commands/capture/screenshot.rs)
is the same four items with a `Deferred` handler — see [Calling it](#calling-it).

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

Replies stay small and scoped. The surface this one replaced had a `query_state` tool
that answered with the entire app state — tens of thousands of tokens, mostly irrelevant
to the question asked, filling the caller's context. That dump was deleted; the trap it
stands for did not go away — one convenient dump instead of many scoped reads. A command
answers the one question it names; if a reply could run to pages, the command is too
broad — split it the way a player's perception is split: which screen, what has focus,
what is in view.

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
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppPhaseArgs {}
```

`deny_unknown_fields` is not optional. It is what turns "you sent a field I do not have"
into a `BadArguments` answer that NAMES the offending field and carries this type's own
published shape, instead of a silently ignored key. The published shape cannot carry the
attribute — the deserializer is never told about it — so what the strictness buys is the
refusal itself, on the first call that gets it wrong.

`Debug` is required by the trait: a decoded call waits in a `PendingQueue` whose deadline
sweep logs the payload of anything that timed out unclaimed.

### The reply type

```rust
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AppPhaseReply {
    /// Where the app is, at every level of its state machine.
    phase: AppPhaseNet,
}
```

The reply is where a command's DOMAIN REFUSALS go — "the target is not adjacent", "there is
no selected ganger". `CommandOutcome` has no `Refused` variant on purpose: a command that
runs and says no does so inside its own declared reply type, whose shape the catalogue
publishes.

Anything the reply embeds must derive `Deserialize` too — that is the impl the published
shape is read out of. The game's five state enums do not, and deliberately are not made to:
`crates/gdtf_app/src/states/` stays free of wire derives, and
[`commands/../wire/phase.rs`](../../crates/gdtf_app/src/dev/net_qa/wire/phase.rs) mints wire
MIRRORS of them instead, with a wildcard-free `from_state` per level so a new state variant
fails to compile until its mirror gains an arm.

### The command

```rust
impl QaCommand for AppPhase {
    type Args = AppPhaseArgs;
    type Facts = GameFacts;
    type Parked = ();
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
  inside `take_calls`, `Deferred` if it parks its responder and answers on a later frame —
  in `DeferredReplies`, or in a pipeline queue of its own, as `capture.screenshot` parks
  its responder in `CaptureQueue`.
- **`Parked`** is what a call carries while it waits, and is `()` for every command whose
  waiters all settle together — an `Immediate` one included. `wait` is the reason it exists:
  each of its parked calls holds the condition it asked for, so
  `DeferredReplies::answer_resolved` releases only the ones whose condition has come true and
  leaves the rest parked.
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

An ordinary Bevy system with ordinary system params. It never sees the wire text and never
sees a raw `Responder`: it gets `C::Args` in and answers `&C::Reply` out, so the shape a
client was promised in the catalogue is the only shape it can produce.

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

- **The set.** `assert_game_command_set_is_conformant()` runs the per-host assertions —
  unique names, parseable shapes, one body per type name, and a deferral budget that expires
  before the socket stops waiting for the reply — over the real slice. It is already
  registered; a new command is covered by it the moment it joins the list.
- **The command.** Add a case file per command under `crates/gdtf_app/tests/net_qa/` and declare
  it in that directory's `main.rs`. One file is the usual shape —
  [`settings_read.rs`](../../crates/gdtf_app/tests/net_qa/settings_read.rs) and
  [`battle_start.rs`](../../crates/gdtf_app/tests/net_qa/battle_start.rs) — and a command with
  several kinds of case gets a directory instead, as `wait` does in
  [`tests/net_qa/wait`](../../crates/gdtf_app/tests/net_qa/wait).
  The shared `exchange` helper takes a fixture, negotiates, and
  sends over a real socket into the real router, so a case there exercises the whole path a
  live client drives. Assert on the reply rather than on published shape TEXT: the shape is
  traced from the type, so pinning it re-states the type instead of testing behaviour.
  [`crates/gdtf_app/tests/net_qa/commands.rs`](../../crates/gdtf_app/tests/net_qa/commands.rs)
  is for the cases that span the whole host — the catalogue listing, an unknown name, the
  deferral budget — and grows by a name in those lists, not by a per-command case.

## Calling it

From an MCP client, two tools and no more:

```text
commands(host="game")                                    # what can I call?
commands(host="game", command="app.phase", detail="Full") # what does it take?
run(host="game", command="app.phase", arguments="()")     # do it
run(host="game", command="capture.screenshot", arguments="(name: Some(\"menu\"))")
run(host="game", command="battle.start", arguments="(seed: Some(42))")
run(host="game", command="wait", arguments="(condition: BattleDecided)")
```

The game offers seventeen commands today: `app.phase`, `capture.screenshot`, `settings.read`,
`ui.focus`, `playback.state`, `battle.roster`, `battle.turn`, `battle.selection`,
`battle.offers`, `battle.inspect`, `battle.sightline`, `battle.visible`, `log.read`,
`battle.start`, `battle.flee`, `procgen.step` and `wait`.

`settings.read`, `ui.focus` and `playback.state` are the shell reads — they take `()`, are
`Immediate`, and answer before a battle: `settings.read` reports the Options values,
`ui.focus` reports the focused widget and the widgets the current screen registered as
focusable, and `playback.state` reports whether the screen has caught up with the act log.

The eight battle reads are `Immediate` too, and report only what the player can see.
`battle.roster` lists every player card plus the enemies the squad can currently see — the
reduction is which enemies appear, not which fields a card carries, since the stat block
already draws a visible enemy's whole card —
`battle.turn` names the acting gang and the player's, `battle.selection` reports the
selected shooter, its fire mode and the hovered and pinned inspect cells, `battle.offers`
lists the contextual buttons the panel is showing with the target each would act on,
`battle.inspect` reads one cell exactly as the inspect panel draws it, `battle.sightline`
answers whether the squad can see a cell and whether the selected shooter could engage it,
`battle.visible`
lists the enemies, doors and cover inside the lit area, and `log.read` returns the tail of
the act log with a `cap` on how many lines.

Every read that answers a fog question reads the same playback-gated shadows the panels
read — the shown fog, the shown occupancy grid and cover ledger, and each ganger's drawn
cell and drawn vitals. That covers `battle.roster`, `battle.inspect`, `battle.visible` and
the `can_see` half of `battle.sightline`. A ganger is therefore reported on the cell its
sprite stands on, and counts as seen or hidden from that cell, not from the one the sim
has already moved it to, and no read ever reports fog the screen has not drawn yet. The
rest read live state on purpose, because what they report is not drawn from a shadow:
`battle.turn` and `battle.offers` report resources the sim and the panel write each frame,
the `can_engage` half of `battle.sightline` asks the sim's own firing-arc gate,
`battle.selection` reports the pointer's live selection, and `log.read` reads the act log
with no fog filter, matching a combat log that filters none either.

Each read refuses `Unavailable` with code `WrongState` off the battle screen.
`battle.sightline` further needs the battle's running phase, and `battle.offers`,
`battle.inspect` and `battle.visible` need a running battle whose sim state is loaded —
without it the offer and inspect systems never run, so answering would report "nothing
offered" where the truth is "not computed yet". Those three words are pinned command by
command in
[`commands/read/test/availability_words.rs`](../../crates/gdtf_app/src/dev/net_qa/commands/read/test/availability_words.rs).

`capture.screenshot` writes a PNG of what the game is showing and answers `Ran` with a
`ReplyAttachment(Png, …)` naming it; `name` is optional and becomes the file stem inside
the host's shot directory. It is `Deferred`, so the reply arrives once the PNG is on disk;
a capture that never lands answers `Timeout`, and one aimed at an image nothing renders
into is refused `Unavailable` with code `WrongState`. The capture pipeline itself — queue,
settle, aim check, spawn, verify — lives in `crates/gdtf_screenshot` and is shared by both
hosts; the command in
[`crates/gdtf_app/src/dev/net_qa/commands/capture/screenshot.rs`](../../crates/gdtf_app/src/dev/net_qa/commands/capture/screenshot.rs)
only maps its `CaptureOutcome` onto the wire.

`battle.start` and `battle.flee` are the two ends of a battle, and each takes the same path
the button does — `battle.start` writes the message the Battlescape button writes, and
`battle.flee` inserts the same marker the Flee button inserts. Both are `Deferred`.
`battle.start` needs the Menu; its only argument is the seed, which is optional. Omit it and
the game resolves its own; either way the reply carries the seed the battle actually used,
read back from the record generation keeps in
[`crates/gdtf_app/src/states/running/game/battlescape/generation/battle_sim/resolved.rs`](../../crates/gdtf_app/src/states/running/game/battlescape/generation/battle_sim/resolved.rs).
The reply arrives once generation has finished, so the app is already in the battle.
`battle.flee` needs a battle in its running phase and answers once the battle has left it.

`procgen.step` advances staged generation by one stage, writing the same request the stepper
panel's Next button writes. It is `Immediate` and needs the procgen stepper to own a
situation that is still generating. The stepper is a `dev_tools` build option: the command is
published either way, so the command list is the same in every build, but a binary built
without `dev_tools` refuses it `Unavailable` with code `NotBuilt` rather than dropping the
name.

`wait` holds its reply until one named condition becomes true, so an agent can stop polling.
It is `Deferred` with a two-minute budget, and answers `Timeout` — never a refusal — when the
condition never comes. The channel serves one caller at a time, so a parked `wait` holds it
until it answers and any other connection meanwhile is told `Busy`. The seven conditions are
`CaughtUp` (the playback gate is open),
`Phase` (the live phase matches every level the caller named; levels left out are wildcards,
so `(condition: Phase((game: Some(BattleScape))))` waits for the battle map whatever the
battle is doing), `LogAtLeast` (the act log holds at least N entries), `WalkComplete` (nobody
is part-way through a walk), `TurnChanged` (a turn hand-off after the call was admitted),
`BattleDecided` (the battle has been decided — by an outcome or by fleeing) and
`GenerationComplete` (the situation has finished generating). The last two read markers the
app clears as it leaves the phase that set them, so ask for them while that phase is still
live: a `BattleDecided` asked once the battle has moved on to the aftermath answers `Timeout`,
and so does a `GenerationComplete` asked once the battle map is up. A name that is not one of
those seven fails to deserialize and comes back as `BadArguments` with `wait`'s own argument
shape attached. The conditions and what each resolves against live in
[`crates/gdtf_app/src/dev/net_qa/commands/wait/probe.rs`](../../crates/gdtf_app/src/dev/net_qa/commands/wait/probe.rs).

Args, replies and published shapes are all **RON**. `arguments` is a string of compact RON
shaped by that command's own `schemas.arguments`; a command that takes none is `"()"`. The
MCP envelope itself stays JSON-RPC — the RON is opaque text riding inside it.

`commands` reads the catalogue from the RUNNING host, so its `availability` column is the
live answer rather than a static claim. Four things can go wrong, and each tells you how to
fix it in one round trip: `Unknown` lists every name the host does offer, `BadArguments`
carries the schema your body failed against, `Unavailable` names the precondition that is
missing, and a rider this build has not implemented (`await_ready`, `capture`) is refused
`Unavailable` with code `NotBuilt` rather than run without it. `BadArguments` names the
offending field in its `detail`, which is what `deny_unknown_fields` really buys.

## Why the shape is what it is

**A host publishes ONE list of typed commands, and the wire carries any command in two
variants that never change.**

- A command is a unit struct implementing `QaCommand` (`crates/gdtf_qa_command`): two
  associated types whose RON shapes are **traced out of their own `Deserialize` impls** —
  the same impls that decode the wire, so a published shape cannot disagree with the
  decoder — a name, a summary, a declared timing, a deferral budget, a pure availability
  predicate over that host's facts type, and the registration of an ordinary Bevy system.
- A host owns one `&[&dyn ErasedCommand<F>]` — the game's is `GAME_COMMANDS` in
  `crates/gdtf_app/src/dev/net_qa/commands/set.rs`. The catalogue walk, the admission scan
  and the registration walk all read that same slice, so **a command cannot be advertised
  without being admissible and wired**.
- The wire surface is exactly two request variants (`Catalogue`, `Run`) and two response
  variants (`Catalogue`, `Outcome`). A command is *data* inside them, so adding one moves
  no protocol version and adds no variant.
- The MCP courier exposes exactly two tools, `commands` and `run`. **Neither names a
  command**, in its schema or its description. A client discovers what it can call by
  calling `commands` against the running host, and reads the RON shapes the host traced
  from its own Rust types — one notation end to end, since the value it writes back is RON
  too.
- **Exactly one system per host drains the request inbox.** `NetInbox::drain()` takes
  everything in the channel, so a second router would swallow the first one's requests
  nondeterministically. The command layer widens the existing router with `Catalogue` and
  `Run` arms rather than registering its own.
- A command's preconditions are answered by its availability predicate, as
  `Unavailable { code, note }` — not by route-time state gates. The router can only ever
  answer something blunter, and a per-command precondition is per-command knowledge.

**What this buys:** adding a command needs no courier rebuild, no protocol bump, and no
MCP reconnect. That property is the reason for the shape, and losing it is the signal that
a change has gone wrong.
