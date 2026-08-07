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

The game offers forty-nine commands today: `app.phase`, `capture.screenshot`,
`settings.read`, `ui.focus`, `playback.state`, `battle.roster`, `battle.turn`,
`battle.selection`, `battle.offers`, `battle.inspect`, `battle.sightline`, `battle.visible`,
`battle.cost`, `log.read`, `battle.start`, `battle.flee`, `procgen.step`, `wait`, `act.select`,
`act.select_next`, `act.select_prev`, `act.select_clear`, `act.move`, `act.fire`,
`act.reload`, `act.set_stance`, `act.set_aiming`, `act.set_facing`, `act.end_turn`,
`act.melee`, `act.shove`, `act.stabilize`, `act.execute`, `act.throw_grenade`,
`act.open_door`, `act.enter_emplacement`, `act.exit_emplacement`, `input.press_key`,
`input.hover`, `input.set_focus`, `input.focus_step`, `input.activate`,
`input.click_cell`, `view.level_up`, `view.level_down`, `view.toggle_full_view`,
`view.pan`, `view.look_at` and `battle.set_fire_mode`.

`settings.read`, `ui.focus` and `playback.state` are the shell reads — they take `()`, are
`Immediate`, and answer before a battle: `settings.read` reports the Options values,
`ui.focus` reports the focused widget and the widgets the current screen registered as
focusable, and `playback.state` reports whether the screen has caught up with the act log.

The nine battle reads are `Immediate` too, and report only what the player can see.
`battle.roster` lists every player card plus the enemies the squad can currently see, each
card naming the cell its sprite stands on — the
reduction is which enemies appear, not which fields a card carries, since the stat block
already draws a visible enemy's whole card —
`battle.turn` names the acting gang and the player's, `battle.selection` reports the
selected shooter, its fire mode and the hovered and pinned inspect cells, `battle.offers`
lists the contextual buttons the panel is showing with the target each would act on,
`battle.inspect` reads one cell exactly as the inspect panel draws it, `battle.sightline`
answers whether the squad can see a cell and whether the selected shooter could engage it,
`battle.visible`
lists the enemies, doors and cover inside the lit area, `battle.cost` quotes what one act
would charge one ganger in TU and whether the sim would allow it — every number is the
sim's own cost helper and every verdict its own legality check, and nothing in the battle
moves — and `log.read` returns a window of
the act log — `since` picks where it starts, `cap` how many lines it keeps, and the reply
brackets the window with the log's `head` and `oldest` so a caller can page.

`log.read` is where the whole-log dump would come back if it were allowed to, so its cap
is bounded rather than obeyed: absent it is 50, and any cap a caller names is clamped to
200. The ring buffer holds 2048 lines. A caller that wants more than one window pages with
`since`, taking the previous reply's `head` as the next cursor, and compares its cursor
against `oldest` to see whether the buffer threw lines away in between.

Every read that answers a fog question reads the same playback-gated shadows the panels
read — the shown fog, the shown occupancy grid and cover ledger, and each ganger's drawn
cell and drawn vitals. That covers `battle.roster`, `battle.inspect`, `battle.visible` and
the `can_see` half of `battle.sightline`. A ganger is therefore reported on the cell its
sprite stands on, and counts as seen or hidden from that cell, not from the one the sim
has already moved it to, and no read ever reports fog the screen has not drawn yet. The
rest read live state on purpose, because what they report is not drawn from a shadow:
`battle.turn` and `battle.offers` report resources the sim and the panel write each frame,
the `can_engage` half of `battle.sightline` asks the sim's own firing-arc gate,
`battle.selection` reports the pointer's live selection, `battle.cost` prices from the live
tuning, grids, squad fog, cover and ganger state the act itself would be charged against —
a quote taken while playback is behind is what the sim would charge now, not what the
sprite's cell suggests — and `log.read` reads the act log
with no fog filter, matching a combat log that filters none either.

Each read refuses `Unavailable` with code `WrongState` off the battle screen. `log.read`
is the one whose phase gate leaves a window open — the battlescape is up while the battle
is still generating, and the act log arrives with the rest of the battle runtime — so a
call inside that window is refused `MissingModel` rather than answered with an empty log
that would read as "nothing has happened".
`battle.sightline` and `battle.cost` further need the battle's running phase — `battle.cost`
answers `MissingModel` inside that phase whenever the tuning, grids, fog and cover it
prices from are not loaded — and `battle.offers`,
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

Eleven of the nineteen `act.*` commands are the classic acts a player takes with the keyboard
and the
pointer: `act.select`, `act.select_next`, `act.select_prev`, `act.select_clear`, `act.move`,
`act.fire`, `act.reload`, `act.set_stance`, `act.set_aiming`, `act.set_facing` and
`act.end_turn`. Each pushes an `ActIntent` onto
[`PendingActIntent`](../../crates/gdtf_battle_input/src/act_bus/intent/seam/queue.rs), the
same bus the keybinds and the pointer push onto, and `dispatch_act_intents` writes the sim
message. **No act command writes to the sim itself**, so legality stays where it already
lives: a shot the sim declines is never declared, and a move it refuses is logged as
`MoveRefused` with the sim's own reason.

The three `set_*` commands take an absolute value rather than cycling the way the keybind
does, so the same call twice leaves the same state — that is what makes them driveable
without reading the current value first.

Ten of the eleven are `Immediate`. The claim system pushes the intent and records the act
log's head, and a settle system ordered after `SimSystems::Record` answers in the same frame
with `ActReply::Accepted { from_seq, to_seq, complete }`: `from_seq..to_seq` is the half-open
act-log window the call opened, and `complete` is false while the actor is still walking the
act out. Read the window with `log.read` to see what the sim actually did — the deed kinds
inside it are the evidence, and a call the sim declined comes back with `from_seq` equal to
`to_seq`. The four selection commands answer `SelectReply` instead, which names who is
selected and carries no sequence numbers.

**A cleared selection does not stay cleared.** The game always hands the player someone to
act with:
[`auto_select_first_player_ganger`](../../crates/gdtf_battle_input/src/pointer/selection/auto_select.rs)
re-selects the first living player ganger by cell order whenever nothing is selected, and
the clear keybind is undone the same way. So `act.select_clear` answers
`SelectReply::Selected { shooter: None }` for the frame the clear landed in, and by the next
frame someone is selected again. Every act runs on the live selection, so a call after a
clear acts through whichever ganger the game re-picked. Name the actor with `act.select`
before an act; do not treat the empty reply as a lasting state.

`act.end_turn` is the one `Deferred` command of the set. Its reply is held until the turn
comes round to the player again, so it lands after the enemy has finished acting and its
window brackets everything that happened in between. Its budget is 30 seconds. Its `complete`
is always true, and carries no information: the command names no actor, so the settle watches
nobody's walk. It is not a claim that every walk has finished. Do not branch on it — branch on
the window, or wait on `WalkComplete` if a walk is what you are waiting for.

The QA layer invents no refusal vocabulary of its own: `ActRefusalNet` has exactly three
variants, `UnknownToken` (a `GangerToken` naming no living ganger), `NoShooter` (the act
needs a selection and there is none) and `NoOffer` (the contextual panel is offering nothing
for that act family). All three are conditions the QA layer can see before the sim is
involved. Everything else the sim decides, and the act log reports.

All eleven need a running battle whose screen has caught up with the act log — `Running` plus
`Caught`. Off the battle screen they refuse `WrongState`; while the screen is still replaying
they refuse `Replaying`, because the act bus drops an intent whose gate is shut. Ten of them
ask for nothing more. `act.end_turn` also needs the turn to belong to the player, and refuses
`WrongState` while another faction is acting: no player path ends someone else's turn — there
is no button and no key for it — so allowing it over the wire would drive the sim somewhere
real play cannot go. `wait` with `TurnChanged` is how a run gets past an enemy turn.

The other eight `act.*` commands are the contextual acts — `act.melee`, `act.shove`,
`act.stabilize`, `act.execute`, `act.throw_grenade`, `act.open_door`,
`act.enter_emplacement` and `act.exit_emplacement`. They live in
[`crates/gdtf_app/src/dev/net_qa/commands/act/contextual/`](../../crates/gdtf_app/src/dev/net_qa/commands/act/contextual)
and they all take `()`.

**None of them takes a target, because no press does either.** The contextual panel offers
exactly one target per act family, written by that family's offer scan, and a mouse click or
a digit slot key pushes whatever the offer holds. A command that took a target could do
something no player can. So an agent chooses by moving and selecting and then reading
`battle.offers` — exactly as a player chooses by moving and looking at the panel. Each
command pushes the offered target onto the same `PendingContextualIntents` queue the button
pushes onto, and the sim's own `dispatch_*` is the authoritative gate: nothing on the QA side
re-checks adjacency, faction, life state or TU.

`act.throw_grenade` is the one that needs more than a selection: its offer scan reads the
hovered cell, and it only offers while the shooter wields an arcing ranged weapon —
[`crates/gdtf_app/tests/contextual_panel/throw.rs`](../../crates/gdtf_app/tests/contextual_panel/throw.rs)
pins both halves, and pins that a press aims at the hovered cell. On a host with a primary
window the drive is `input.hover`, then `battle.offers` to see which cell came out, then
`act.throw_grenade`. Each of those lands on its own frame, which is what makes it work:
`ContextualPanelSystems::Offer` is ordered *before* `pick_hovered_cell`, so within any one
frame the offer scan reads the cell the frame before resolved, never the pixel that arrived
this frame. With nothing hovered — including every host built without a primary window — the
command refuses `NoOffer`.

They are `Immediate`, they need the same `Running` plus `Caught` state the classic acts need,
and they answer `ContextualReply`: `Accepted { from_seq, to_seq, complete, target }`, where
`target` is the same `OfferTargetNet` `battle.offers` reported, or
`Refused { reason: NoOffer }` when the family had nothing on offer. There is no unaffordable
or illegal refusal: every contextual dispatch in the sim rejects by doing nothing and
emitting nothing, so a call the sim declined comes back with `from_seq` equal to `to_seq`.

**An empty window is not always a decline.** `ActDeed` has no variant for opening a door or
entering and leaving an emplacement, so `act.open_door`, `act.enter_emplacement` and
`act.exit_emplacement` answer an empty window even when they worked. Read the world for those
three — the door's open state, the emplacement's occupancy — not the log.

`act.fire` is the one act that needs more than the selection to build its request: the live
`SelectedFireMode` and `CombatTuning`. A host holding neither cannot consider the shot at
all, so it refuses `MissingModel` naming the absent resource — the same call `log.read`
makes when the act log has not arrived. Without that refusal the reply would be an empty
window, which is exactly what a shot the sim declined answers, and a caller could not tell
a real sim decision from a host that was not loaded.

The six `input.*` commands drive the raw input paths, for the screens and the moments where a
named act is not enough. Each one writes what the real device writes and lets the game decide
what that means.

`input.press_key` writes the same `KeyboardInput` message `bevy_winit` writes, which
`keyboard_input_system` folds into `ButtonInput<KeyCode>` in `PreUpdate` — so every consumer
sees the press on the next frame, keybinds and focus bridge alike. It takes either a physical
key or a named action; a named action is resolved through the live `Keybinds` resource, so a
rebind is honoured and the reply names the physical key that was actually pressed. It is the
one `Deferred` command of the six: the reply is held until the matching release has been
written, so the caller's next command sees a settled keyboard rather than a key stuck down. A
named action asked for before the keybind table has loaded is refused `MissingModel`.

`input.hover` moves the pointer to a pixel: the cursor goes into the primary window and a
`CursorMoved` message goes out, which also takes pointer ownership back from the gamepad. It
never resolves a cell — projecting a pixel onto a cell belongs to `pick_hovered_cell`, and the
reply echoes the pixel only. Read the resulting cell with `battle.selection`. A host with no
primary window refuses `WrongState` naming it.

Whether that pixel becomes a cell is `pick_hovered_cell`'s call, and it resolves `None` unless a
`WorldCamera` and a primary window are both there, the cursor sits off the UI and inside the
viewport, and the pixel lands on the grid.
[`crates/gdtf_battle_input/tests/picking/resolve.rs`](../../crates/gdtf_battle_input/tests/picking/resolve.rs)
asserts the projection and four of those refusals: no cursor position, an off-grid pixel, no
`WorldCamera`, and no primary window. That last one resolves a cell first and then despawns the
window, so it also catches a pick that holds the cell it last resolved instead of dropping it.
[`crates/gdtf_battle_input/tests/picking/viewport.rs`](../../crates/gdtf_battle_input/tests/picking/viewport.rs)
asserts the other two, a cursor outside the viewport rect and a cursor over a HUD panel.
`input.click_cell` is no substitute: it
writes the hovered cell directly and runs before that same pick, which overwrites it inside the
frame, so only the cell it pins survives.

**`hovered` needs a primary window; the pinned cell does not.** Drive the running game with
`input.hover` and then read `battle.selection`: a pixel over the grid comes back as a cell, and
a pixel over the UI or off the grid comes back `None`, because the pick fails closed rather
than holding the cell it last resolved. The battle cases under
[`crates/gdtf_app/tests/net_qa/`](../../crates/gdtf_app/tests/net_qa) run on one of two headless
harnesses and neither has a primary window: `battle_app_listening` builds on
`GdtfLoadTestAppBuilder`, which sets `primary_window: None`, and
`battle_fixture::menu_app_with_net_qa` builds on `GdtfTestAppBuilder`, which is `MinimalPlugins`
and never adds `WindowPlugin` at all. So on those hosts `input.hover` refuses `WrongState` and
`battle.selection` reports `hovered: None` on every read, and they assert the pinned cell instead.
The screenshot cases in that same directory build their own windowed app, so the limit is the
harness rather than the wire.

`input.set_focus`, `input.focus_step` and `input.activate` are the focus bridge. `set_focus`
puts focus on one widget named by a token from `ui.focus`'s focusable list, and refuses
`WrongState` for a token the current screen never registered — focus can never land somewhere
the navigation map cannot leave. `focus_step` writes one navigate request: `Next` and `Prev`
are the down and up edges, `Left` and `Right` the west and east ones; a step with no neighbour
that way is swallowed exactly as the real one is, so compare the focus the reply reports
against the focus before the call. `activate` writes the activation message Enter and the
gamepad's south button write, always on whatever holds focus and never on a caller-named
target, and refuses `WrongState` when nothing is focused. All three run in the same frame band
the real keyboard bridge runs in, so the screen has acted by the time the reply lands.

`input.click_cell` is the left click. It makes the named cell the hovered inspect target and
then runs the game's own decide-then-apply pair — `decide_left_click`, `decide_pin`,
`apply_left_click`, `apply_pin` — so one click may select a ganger, pin a move target, confirm
the move or fire, and the decision is the game's rather than the caller's. It is not a side
door into the sim: an act it decides goes onto the same `PendingActIntent` bus every act does
— a click that only selects a ganger or pins a target pushes nothing — it answers the same
`ActReply` window, and it needs the same `Running` plus `Caught` state every classic act
needs. It carries no turn-owner check of its own: that one is `act.end_turn`'s alone.

The six view and battle controls all need a running battle with its sim state loaded, and
each takes the path the matching keyboard or panel control takes. `view.level_up`,
`view.level_down` and `view.toggle_full_view` push `LevelUp`, `LevelDown` and
`ToggleFullView` onto the same `PendingActIntent` bus the level and full-view keys push, and
answer `Immediate` with the storey and view mode the frame settles on — stepping past the
ground floor or the top storey clamps and the reply reports the unchanged value. Neither
needs a caught-up screen, because the view moves while the act log is still playing back.
`view.pan` moves the camera by a signed cell offset and `view.look_at` centres it on a cell;
both write the transform through the one function every camera mover uses, both are
`Deferred` until the frame's bounds clamp has run, and both then answer with the cell the
camera actually ended on — absent when it was left off the grid. `view.pan` is not a second
way to hold W: the keys move by speed times frame time, while the offset asked for here is
the offset taken. `battle.set_fire_mode` writes the same `SelectedFireMode` resource the
action bar's mode panel writes, through the same lookup, and refuses `MissingModel` with a
note saying which precondition is missing when nothing is selected, the shooter holds no
gun, or the gun does not offer that mode.

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
