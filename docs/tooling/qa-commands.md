---
name: Adding a QA command
description: How to add a command to either QA host — a file, one line in that host's list, and a test — written from app.phase on the game and editor.phase on the editor.
---

# Adding a QA command

A QA command is how an agent drives or reads a running GDTF process. Adding one is **a
file, one line in a host's list, and a test**. It moves no protocol version, adds no wire
variant, and changes nothing in the MCP courier — because a command is DATA carried inside
frozen envelope variants rather than a variant of its own
(see [Why the shape is what it is](#why-the-shape-is-what-it-is) below).

**There are two hosts, and they take the same three edits.** The game is
`host="game"` and the editor is `host="editor"`; each owns its own command list, its own
facts type and its own wire mirrors, so a command is written into one host and cannot
compile into the other.

Everything below is written from the simplest command the game publishes,
[`crates/gdtf_app/src/dev/net_qa/commands/read/app_phase.rs`](../../crates/gdtf_app/src/dev/net_qa/commands/read/app_phase.rs).
Read that file alongside this one: every shape shown here is in it, at that path. Nothing
here describes a command nobody has written. The game's other reads — `settings.read`,
`ui.focus` and `playback.state`, beside it under `commands/read/` — are the same items
reading a different resource, and
[`commands/capture/screenshot.rs`](../../crates/gdtf_app/src/dev/net_qa/commands/capture/screenshot.rs)
is the same items with a `Deferred` handler — see [Calling it](#calling-it). The editor's
[`commands/read/editor_phase.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/read/editor_phase.rs)
is the same items again on the other host.

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

## The edits

1. **A file** under the host's `commands/` directory — one command per file, grouped by
   what it does (`read/` for a command that answers from the world without changing it).
2. **One line** in that host's list. The game's is `GAME_COMMANDS` in
   [`commands/set.rs`](../../crates/gdtf_app/src/dev/net_qa/commands/set.rs) and the
   editor's is `EDITOR_COMMANDS` in
   [its own `commands/set.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/set.rs);
   the whole entry is `&YourCommand`.
3. **A test.** The suite for the game's command layer is
   [`crates/gdtf_app/tests/net_qa/commands.rs`](../../crates/gdtf_app/tests/net_qa/commands.rs).
   The editor has seven. Hello and the lifecycle commands are in
   [`crates/gdtf_content_editor/tests/net_qa_hello/`](../../crates/gdtf_content_editor/tests/net_qa_hello),
   the shared reads are in
   [`crates/gdtf_content_editor/tests/net_qa_editor_reads/`](../../crates/gdtf_content_editor/tests/net_qa_editor_reads),
   the theme helpers and the Injury sub-tab write are in
   [`crates/gdtf_content_editor/tests/net_qa_editor_commands/`](../../crates/gdtf_content_editor/tests/net_qa_editor_commands),
   the draft writes over every form tab are in
   [`crates/gdtf_content_editor/tests/net_qa_editor_forms/`](../../crates/gdtf_content_editor/tests/net_qa_editor_forms),
   the prefab-canvas commands are in
   [`crates/gdtf_content_editor/tests/net_qa_editor_prefab/`](../../crates/gdtf_content_editor/tests/net_qa_editor_prefab),
   the Injury weighting table is in
   [`crates/gdtf_content_editor/tests/net_qa_editor_weighting/`](../../crates/gdtf_content_editor/tests/net_qa_editor_weighting),
   and one authoring session end to end is in
   [`crates/gdtf_content_editor/tests/net_qa_editor_authoring/`](../../crates/gdtf_content_editor/tests/net_qa_editor_authoring).
   All seven build their app and their client from
   [`crates/gdtf_content_editor/tests/net_qa_shared/`](../../crates/gdtf_content_editor/tests/net_qa_shared),
   which each target includes by `#[path]`, and the four that answer during the Load pass take
   that case from there too. All of them use a real socket, a real listener,
   and the real router.

## The file

The pieces: the argument type, the reply type, the unit struct, and the handler.

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
shape is read out of. The game's state enums do not, and deliberately are not made to:
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
  and whether a `Run` is admitted. They cannot disagree. `app.phase` is always
  `Available` because a command that reports where the app is has to be answerable wherever
  the app is; a command with a precondition returns
  `CommandAvailability::Unavailable { code, note }`, naming the missing thing in words a
  caller can act on.
- **`register_handler`** puts the handler wherever in the schedule its work belongs.
  `.after(QaCommandSystems::Claim)` is the floor — that is where the decode step fills
  the queue, so a handler that ran earlier would answer every call a frame late. A handler
  that WRITES something the game also writes needs an edge against each of those systems
  too. Without one the built schedule picks the winner, and the reply can name a value that
  another system overwrote later in the same frame. Where one value has several writers and
  several readers, a `SystemSet` per writer, all inside one set the readers order against,
  costs less than an edge per pair.
  A weapon's chosen fire mode is the worked example: `FireModeSystems` (in
  `gdtf_battle_input`) holds `Panel`, then `Command`, both inside `FireModeSystems::Write`.
  The action bar's mode panel and `battle.set_fire_mode` each join one of them, so a QA call
  and a panel press on the same frame settle in a fixed order — the call last — instead of
  whichever way the build happened to sort them. A reader takes
  `.after(FireModeSystems::Write)` and needs no edge per writer: `act.fire`,
  `battle.selection`, the panel's active-segment sync and the fire-target population all do.

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

Both layers are cheap:

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

From an MCP client, these tools and no more:

```text
commands(host="game")                                    # what can I call?
commands(host="game", command="app.phase", detail="Full") # what does it take?
run(host="game", command="app.phase", arguments="()")     # do it
run(host="game", command="capture.screenshot", arguments="(name: Some(\"menu\"))")
run(host="game", command="battle.start", arguments="(seed: Some(42))")
run(host="game", command="wait", arguments="(condition: BattleDecided)")

# Every editor line below also carries instance="<id>", naming the editor its
# launch reply reported; an editor call that names no instance is refused.
commands(host="editor")                                   # the other host, same tools
commands(host="editor", command="editor.phase", detail="Full")
run(host="editor", command="editor.phase", arguments="()")
run(host="editor", command="editor.set_mode", arguments="(mode: Armor)")
run(host="editor", command="editor.new", arguments="(mode: Armor)")
run(host="editor", command="editor.load_armor", arguments="(key: \"flak_vest\")")
run(host="editor", command="editor.save", arguments="(mode: Armor)")
run(host="editor", command="editor.last_save", arguments="()")
run(host="editor", command="editor.validation", arguments="()")
run(host="editor", command="editor.families", arguments="()")
run(host="editor", command="editor.families", arguments="(family: Some(Armor))")
run(host="editor", command="editor.session", arguments="()")
run(host="editor", command="editor.draft", arguments="()")
run(host="editor", command="editor.set_mode", arguments="(mode: Terrain)")
run(host="editor", command="editor.set_field", arguments="(field: Terrain(Kind(Emplacement)))")
run(host="editor", command="editor.list_op", arguments="(list: EntrySides, op: Toggle(EntrySide(East)))")
run(host="editor", command="editor.set_field", arguments="(field: Armor(Floor(part: Head, value: 12)))")
run(host="editor", command="editor.list_op", arguments="(list: SpriteFrames, op: MoveUp(2))")
run(host="editor", command="editor.select_theme", arguments="(key: \"00000000-0000-0000-0000-01840a900001\")")
run(host="editor", command="editor.set_mode", arguments="(mode: Theme)")
run(host="editor", command="editor.toggle_terrain", arguments="(key: \"00000000-0000-0000-0000-01840a910005\")")
run(host="editor", command="editor.set_default_floor", arguments="(key: \"00000000-0000-0000-0000-01840a910004\")")
run(host="editor", command="editor.set_mode", arguments="(mode: Prefab)")
run(host="editor", command="editor.set_grid_size", arguments="(width: 5, height: 5, levels: 3)")
run(host="editor", command="editor.select_tile", arguments="(key: \"00000000-0000-0000-0000-01840a910004\")")
run(host="editor", command="editor.select_facing", arguments="(facing: East)")
run(host="editor", command="editor.set_level", arguments="(level: 1)")
run(host="editor", command="editor.paint", arguments="(x: 2, y: 3)")
run(host="editor", command="editor.map", arguments="(level: 1)")
run(host="editor", command="editor.set_mode", arguments="(mode: Injury)")
run(host="editor", command="editor.select_injury_tab", arguments="(tab: Tables)")
run(host="editor", command="editor.select_weighting_table", arguments="(category: Leg, context: Melee)")
run(host="editor", command="editor.list_op", arguments="(list: WeightingBucket(Minor), op: Add)")
run(host="editor", command="editor.set_field", arguments="(field: Weighting(RowWeight(bucket: Minor, index: 0, weight: 4)))")
run(host="editor", command="editor.weighting", arguments="()")
run(host="editor", command="editor.save_weighting", arguments="()")
run(host="editor", command="editor.set_mode", arguments="(mode: Field)")
run(host="editor", command="editor.new", arguments="(mode: Field)")
run(host="editor", command="editor.set_field", arguments="(field: Field(Damage(3)))")
run(host="editor", command="editor.set_field", arguments="(field: Field(Duration(Turns(2))))")
run(host="editor", command="editor.list_op", arguments="(list: FieldImmuneArmorTypes, op: Toggle(ImmuneArmorType(Flak)))")
run(host="editor", command="editor.save", arguments="(mode: Field)")
run(host="editor", command="capture.screenshot", arguments="(name: Some(\"editor_shot\"))")
run(host="editor", command="wait", arguments="(condition: ChecksComplete)")
run(host="editor", command="wait", arguments="(condition: RegistryRearmed(family: Terrain))")
```

The editor offers thirty-eight commands today. They are listed in
[The editor host](#the-editor-host) below.

The game offers these commands today: `app.phase`, `capture.screenshot`,
`settings.read`, `ui.focus`, `playback.state`, `battle.roster`, `battle.turn`,
`battle.selection`, `battle.offers`, `battle.inspect`, `battle.sightline`, `battle.visible`,
`battle.reachable`, `battle.cost`, `log.read`, `log.omniscient_read`, `battle.start`,
`battle.flee`, `procgen.step`, `wait`, `act.select`,
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

The battle reads are `Immediate` too. `battle.roster`, `battle.inspect`, `battle.visible`,
`battle.reachable`, the `can_see` half of `battle.sightline` and `log.read` are fog-gated and
report only what the player can see. `battle.turn`, `battle.offers`, `battle.selection`,
`battle.cost` and the `can_engage` half of `battle.sightline` read live state on purpose, and
`log.omniscient_read` reads the act log with no filter at all — see the two paragraphs below
for which is which and why.
`battle.roster` lists every player card plus the enemies the squad can currently see, each
card naming the cell its sprite stands on — the
reduction is which enemies appear, not which fields a card carries, since the stat block
already draws a visible enemy's whole card —
`battle.turn` names the acting gang and the player's, `battle.selection` reports the
selected shooter, its fire mode and the hovered and pinned inspect cells, `battle.offers`
lists the contextual buttons the panel is showing, each with the target it would act on and
a `pressable` flag that is false when the panel greys the button out — the actor's TU pool
cannot cover the act's cost, or for melee the actor wields no melee weapon to price a strike
with,
`battle.inspect` reads one cell exactly as the inspect panel draws it — the card for a
squad-visible ganger standing there and the terrain half for the cell itself (its piece kind,
the cover stats the ledger holds, and an emplacement's state and mounted weapon), each absent
when there is nothing to report, and the terrain half absent as well when the fog hides the
cell, so a cell the squad cannot see reports nothing at all — `battle.sightline`
answers whether the squad can see a cell and whether the selected shooter could engage it,
`battle.visible`
lists the enemies, doors and cover inside the lit area — a lit cell reaches the cover list
when the cover ledger holds stats for it, so walls and emplacements are on it too, and a
cell the ledger has no entry for is left off. `battle.reachable` names one ganger and lists
every cell it can walk to inside its TU pool, each entry an `at` and the `cost` reaching it
charges, run as the sim's own reachable search over the cell the sprite is drawn on, the pool
the screen is showing, the grid the screen has drawn and the fog the screen is lighting; an
unknown token is refused `NoSuchGanger` and a ganger the player does not command
`NotYourGanger`, each with no cell list at all, so an empty list only ever means "nowhere to
go" — `battle.cost` quotes what one act
would charge one ganger in TU and whether the sim would allow it — every number is the
sim's own cost helper and every verdict its own legality check, so a melee quote on a
ganger runs the sim's line-of-sight probe as well as its reach check and refuses
`NoLineOfSight` for a target behind cover (a strike on a structure gets no probe, because
the sim runs none either, and is refused `ActNotAllowed` at a cell holding neither cover nor
terrain, because the sim smashes nothing there), and nothing in the battle
moves — and `log.read` returns a window of
the act log — `since` picks where it starts, `cap` how many lines it keeps, and the reply
brackets the window with the log's `head` and `oldest` so a caller can page.

`log.read` is where the whole-log dump would come back if it were allowed to, so its cap
is bounded rather than obeyed: absent it is 50, and any cap a caller names is clamped to
200. The log keeps every line a battle writes for as long as the battle runs, so `oldest` is
always that battle's first line and no cursor a caller holds can be older than it. A caller
that wants more than one window pages with `since`, taking the previous reply's `head` as the
next cursor, and reads `dropped` for how many lines the cap left before the window.

`log.omniscient_read` takes the same arguments over the same log and returns every line
unfiltered and fully identified. Using it is cheating: it is for testing only, and it must
never back a claim about what a player can see. Use `log.read` for that.

Every read that answers a fog question reads the same playback-gated shadows the panels
read — the shown fog, the shown occupancy grid, cover ledger and emplacements, and each
ganger's drawn cell and drawn vitals. That covers `battle.roster`, `battle.inspect`,
`battle.visible`, `battle.reachable` and the `can_see` half of `battle.sightline`. A ganger is
therefore reported on the cell its sprite stands on, and counts as seen or hidden from that
cell, not from the one the sim has already moved it to, and no read ever reports fog the
screen has not drawn yet.
`log.read` answers a fog question from a different place: each line records which gangs
could observe it when it was appended, so the read filters against that fixed record rather
than the fog as it stands now — a line the asking gang could not observe is dropped rather
than blanked, and an actor it could not identify comes back with no token, so a reply may
hold fewer lines than `cap`. The rest read live state on purpose, because what they report
is not drawn from a shadow:
`battle.turn` and `battle.offers` report resources the sim and the panel write each frame,
the `can_engage` half of `battle.sightline` asks the sim's own firing-arc gate and, on the
gun the shooter fires, the sim's own fire-readiness gate — alive, a round in the magazine,
hands free, the time to shoot, and the cell on the grid — so it and `battle.cost {Fire}`
answer alike about one shot,
`battle.selection` reports the pointer's live selection, `battle.cost` prices from the live
tuning, grids, squad fog, cover and ganger state the act itself would be charged against —
a quote taken while playback is behind is what the sim would charge now, not what the
sprite's cell suggests. `log.omniscient_read` is the one read that filters nothing.

`battle.sightline` and `battle.cost {Fire}` agree about one shot because they price one spec,
not one kind. `battle.sightline` takes the kind the gun the shooter fires is set to — a weapon
nothing has been picked for reads as its own single — and `battle.cost {Fire}` takes the kind
named in the call. Each then finds the gun's own `FireMode` entry of that kind and prices and
gates from it, so a call naming the mode the gun is on gets one answer from both. A gun with no
entry of that kind — including one carrying no `FireMode` at all — is refused by both:
`ActNotAllowed` from the quote, `can_engage: false` from the sightline.

Each read refuses `Unavailable` with code `WrongState` off the battle screen. The two log
reads are the ones whose phase gate leaves a window open — the battlescape is up while the
battle is still generating, and the act log arrives with the rest of the battle runtime —
so a call inside that window is refused `MissingModel` rather than answered with an empty
log that would read as "nothing has happened".
`battle.sightline` and `battle.cost` further need the battle's running phase — `battle.cost`
answers `MissingModel` inside that phase whenever the tuning, grids, fog and cover it
prices from are not loaded — and `battle.offers`,
`battle.inspect`, `battle.visible` and `battle.reachable` need a running battle whose sim state
is loaded — without it the offer and inspect systems never run, so answering would report
"nothing offered" where the truth is "not computed yet". `battle.reachable` answers
`MissingModel` inside that phase as well, whenever the shown grid, the shown fog, the player's
faction or the authored terrain it searches over is not there, rather than reporting an empty
set of cells. Those words are pinned command by
command in
[`commands/read/test/availability_words.rs`](../../crates/gdtf_app/src/dev/net_qa/commands/read/test/availability_words.rs).

`capture.screenshot` writes a PNG of what the game is showing and answers `Ran` with a
`ReplyAttachment(Png, …)` naming it; `name` is optional and becomes the file stem inside
the host's shot directory. It is `Deferred`, so the reply arrives once the PNG is on disk;
a capture that never lands answers `Timeout`, and nothing refuses it. The capture pipeline
itself — queue, settle, spawn the readback, verify — lives in `crates/gdtf_screenshot` and
is shared by both hosts. `WindowCapturePlugin` there points the window's output attachment
at an image it owns for the capture frame, reads that image back and writes the PNG, so no
camera is ever retargeted and a covered or minimized window still captures. The command in
[`crates/gdtf_app/src/dev/net_qa/commands/capture/screenshot.rs`](../../crates/gdtf_app/src/dev/net_qa/commands/capture/screenshot.rs)
only maps its `CaptureOutcome` onto the wire.

`battle.start` and `battle.flee` start and end a battle, and each takes the same path
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
until it answers and any other connection meanwhile is told `Busy`. The conditions are
`CaughtUp` (the playback gate is open),
`Phase` (the live phase matches every level the caller named; levels left out are wildcards,
so `(condition: Phase((game: Some(BattleScape))))` waits for the battle map whatever the
battle is doing), `LogAtLeast` (the act log holds at least N entries), `WalkComplete` (nobody
is part-way through a walk), `TurnChanged` (a turn hand-off after the call was admitted),
`BattleDecided` (the battle has been decided — by an outcome or by fleeing) and
`GenerationComplete` (the situation has finished generating). Those last conditions read markers the
app clears as it leaves the phase that set them, so ask for them while that phase is still
live: a `BattleDecided` asked once the battle has moved on to the aftermath answers `Timeout`,
and so does a `GenerationComplete` asked once the battle map is up. A name that is not one of
those fails to deserialize and comes back as `BadArguments` with `wait`'s own argument
shape attached. The conditions and what each resolves against live in
[`crates/gdtf_app/src/dev/net_qa/commands/wait/probe.rs`](../../crates/gdtf_app/src/dev/net_qa/commands/wait/probe.rs).

Some of the `act.*` commands are the classic acts a player takes with the keyboard
and the
pointer: `act.select`, `act.select_next`, `act.select_prev`, `act.select_clear`, `act.move`,
`act.fire`, `act.reload`, `act.set_stance`, `act.set_aiming`, `act.set_facing` and
`act.end_turn`. An act that goes ahead pushes an `ActIntent` onto
[`PendingActIntent`](../../crates/gdtf_battle_input/src/act_bus/intent/seam/queue.rs), the
same bus the keybinds and the pointer push onto, and `dispatch_act_intents` writes the sim
message. **No act command writes to the sim itself**, so legality stays where it already
lives: the game's own guards decide, and the reply names their reason. There are two shapes, below.
`act.fire`, `act.set_stance` and `act.set_facing` ask the sim's own guard first and answer the
reason without pushing anything. `act.move` and `act.reload` always push and read the reason
back out of the act log, so a refused move answers `MoveRefused` and is logged as `MoveRefused`
with that same reason.

The `set_*` commands take an absolute value rather than cycling the way the keybind
does, so the same call twice leaves the same state — that is what makes them driveable
without reading the current value first.

All but one are `Immediate`. The claim system pushes the intent and records the act
log's head, and a settle system ordered after `SimSystems::Record` answers in the same frame
with `ActReply::Accepted { from_seq, to_seq, complete }`: `from_seq..to_seq` is the half-open
act-log window the call opened, and `complete` is false while the actor is still walking the
act out. `Accepted` means the intent reached the sim; read the window with `log.read` to see
what it did.

**An act the game turns down answers a typed reason instead**, one variant per act, so a
client never has to read the log to tell a decline from a success:

| Act | Refusal | Reasons |
| --- | --- | --- |
| `act.fire` | `FireRefused` | `NotAShooter`, `NoFiringWeapon`, `NotAlive`, `Unaffordable`, `MagazineEmpty`, `OutOfBounds`, `NotEnoughHands` |
| `act.reload` | `ReloadRefused` | `AlreadyFull`, `NoTu` |
| `act.move` | `MoveRefused` | `Unreachable`, `Unaffordable`, `Suppressed` |
| `act.set_stance` | `StanceRefused` | `AlreadyHeld`, `Unaffordable` |
| `act.set_facing` | `FacingRefused` | `AlreadyFacing`, `Unaffordable` |

Each reason mirrors the game's own guard — the fire surface's `ShotRefusal`, which is the
sim's `fire_refusal` plus the cases where no request can be built at all, and the sim's
`ReloadOutcome`, `MoveRejection`, `stance_refusal` and `facing_refusal` — so the QA layer
invents nothing. The selection commands answer `SelectReply` instead, which names who is
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

The QA layer invents no refusal vocabulary of its own: `ActRefusalNet` has exactly these
variants, `UnknownToken` (a `GangerToken` naming no living ganger), `NoShooter` (the act
needs a selection and there is none), `NoOffer` (the contextual panel is offering nothing
for that act family) and `TargetMismatch` (the call named a target the panel is not
offering). All of them are conditions the QA layer can see before the sim is
involved. Everything else the sim decides, and the act log reports.

They all need a running battle whose screen has caught up with the act log — `Running` plus
`Caught`. Outside a running battle they refuse `WrongState`. All but one ask for nothing more:
while the screen is still replaying they refuse `Replaying`, because the act bus drops an
intent whose gate is shut. `act.end_turn` also needs the turn to belong to the player: no player
path ends someone else's turn — there is no button and no key for it — so allowing it over the
wire would drive the sim somewhere real play cannot go. `wait` with `TurnChanged` is how a run
gets past an enemy turn.

`act.end_turn` checks the turn owner *before* the screen, so on another faction's turn it
refuses `WrongState` whether or not the screen is behind, and answers `Replaying` only when the
turn is the player's own. The broader refusal wins: waiting for the screen to catch up cannot
make the call work while the enemy is acting, so `Replaying` there would only buy the caller a
retry that refuses again. Outside a running battle it still refuses `WrongState` with the same
"no battle is running" reason the others give — the turn-owner check sits inside the running
battle, not in front of it.

The remaining `act.*` commands are the contextual acts — `act.melee`, `act.shove`,
`act.stabilize`, `act.execute`, `act.throw_grenade`, `act.open_door`,
`act.enter_emplacement` and `act.exit_emplacement`. They live in
[`crates/gdtf_app/src/dev/net_qa/commands/act/contextual/`](../../crates/gdtf_app/src/dev/net_qa/commands/act/contextual)
and all but `act.melee` take `()`. `act.melee` takes `(target: Ganger(...))` or
`(target: Structure(...))` — the same `MeleeTargetNet` `battle.cost {Melee}` prices.

**None of them can act on a target the panel is not offering, because no press can
either.** The contextual panel offers exactly one target per act family, written by that
family's offer scan, and a mouse click or a digit slot key pushes whatever the offer holds
— unless the panel has greyed that button out, which makes it ignore both. So an agent
chooses by moving and selecting and then reading `battle.offers` — exactly as a player
chooses by moving and looking at the panel.

The seven that take `()` act on the offer standing. `act.melee` names the target it is
about to hit and is refused `TargetMismatch` when that is not the one the panel is
offering, so it still does only what a press does. Name it, because the panel's melee scan
falls back to a structure cell when no adjacent ganger is in sight: a client that ignores a
`NoLineOfSight` quote and swings anyway is turned away instead of smashing the wall the
fallback picked. A `Ganger` target matches the offer's `Ganger` and a `Structure` target
matches the offer's `Cell`, so a call names back the target the read named. Each
command pushes the offered target onto the same `PendingContextualIntents` queue the button
pushes onto, and the sim's own `dispatch_*` is the authoritative gate: nothing on the QA side
re-checks adjacency, faction, life state or TU. `battle.offers` does report the panel's own
`pressable` bit, but that is the greyed-out state the button already carries, read off the
same offer resource — not a second legality check living on the QA side. Most of the
scans ask a predicate that folds the cost check in — `can_execute`, `can_stabilize`,
`can_open_door`, `can_throw_grenade`, `can_enter_emplacement`, `can_exit_emplacement`. Melee
and Shove have TU-blind predicates, so they price the act with the sim's own cost helper and
ask `can_spend_tu`. Either way the bit is
the sim's answer about affordability, shown on a button — and, for melee, its answer about
whether the actor wields a melee weapon to price a strike with at all.

`act.throw_grenade` is the one that needs more than a selection: the panel offers
the button while the shooter wields an arcing ranged weapon, and a press aims at
the last cell the cursor hovered —
[`crates/gdtf_app/tests/contextual_panel/throw.rs`](../../crates/gdtf_app/tests/contextual_panel/throw.rs)
and
[`crates/gdtf_app/tests/contextual_panel/throw_hover.rs`](../../crates/gdtf_app/tests/contextual_panel/throw_hover.rs)
pin both halves, and pin that a press after the hover leaves the map still aims
at that last cell. On a host with a primary
window the drive is `input.hover`, then `battle.offers` to see which cell came out, then
`act.throw_grenade`. Each of those lands on its own frame, which is what makes it work:
`ContextualPanelSystems::Offer` is ordered *before* `pick_hovered_cell`, so within any one
frame the offer scan reads the cell the frame before resolved, never the pixel that arrived
this frame. With no cell on offer — including every host built without a primary window,
until a fixture holds a hover — the
command refuses `NoOffer`.

They are `Immediate`, they need the same `Running` plus `Caught` state the classic acts need,
and they answer `ContextualReply`: `Accepted { from_seq, to_seq, complete, target }`, where
`target` is the same `OfferTargetNet` `battle.offers` reported,
`Refused { reason: NoOffer }` when the family had nothing on offer, or, for `act.melee`,
`Refused { reason: TargetMismatch }` when the call named something else. There is still no
unaffordable or illegal refusal on these commands: every contextual dispatch in the sim
rejects by doing nothing and emitting nothing, so a call the sim declined comes back with
`from_seq` equal to `to_seq`. Read `battle.offers` first to tell them apart — an offer
with `pressable: false` is a button on screen the panel has greyed out. The panel makes that
button ignore a click and its digit key; the command does not copy the state, so `act.*`
pushes the target anyway and the sim declines in silence. Every family takes the flag
from the sim — most from a predicate that folds the cost check in, Melee and Shove from
`can_spend_tu` on their own sim cost helper. Melee has a second way to go false:
an actor wielding no melee weapon has no strike to price, so its button greys out too.

They all grey out on a cost the sim charges. `act.execute` and `act.stabilize` are quoted
from `execute_tu_cost` and `stabilize_tu_cost`, and their dispatches debit the actor's pool
by that same quote, so a greyed Execute names an act the sim would decline. Those also
offer nothing at all while the selected ganger is downed or dead, because the predicate they
call reads the actor's life state as well as the target's.

**An empty window still is not proof of a decline for the contextual acts.** `ActDeed` has no
variant for opening a door or entering and leaving an emplacement, so `act.open_door`,
`act.enter_emplacement` and `act.exit_emplacement` answer an empty window even when they
worked. Read the world for those — the door's open state, and the `mounted` flag on the
ganger card `battle.inspect` and `battle.roster` return — not the log.

`act.fire` is the one act that needs more than the selection to build its request: it costs the
shot against the live `CombatTuning`. A host holding none cannot consider the shot at all, so it
refuses `MissingModel` naming that resource — the same call `log.read` makes when the act log has
not arrived. That refusal says the host was never loaded, which is a different answer from the
`FireRefused` a shot the game turned down carries.

The `input.*` commands drive the raw input paths, for the screens and the moments where a
named act is not enough. Each one writes what the real device writes and lets the game decide
what that means.

`input.press_key` writes the same `KeyboardInput` message `bevy_winit` writes, which
`keyboard_input_system` folds into `ButtonInput<KeyCode>` in `PreUpdate` — so every consumer
sees the press on the next frame, keybinds and focus bridge alike. It takes either a physical
key or a named action; a named action is resolved through the live `Keybinds` resource, so a
rebind is honoured and the reply names the physical key that was actually pressed. That table
is there from plugin build — the shipped bindings are compiled in and the RON lands on top of
them — so a named action never waits on the asset. It is the one `Deferred` command among
them: the reply is held until the matching release has been written, so the caller's next
command sees a settled keyboard rather than a key stuck down.

`input.hover` moves the pointer to a pixel: the cursor goes into the primary window and a
`CursorMoved` message goes out, which also takes pointer ownership back from the gamepad. It
never resolves a cell — projecting a pixel onto a cell belongs to `pick_hovered_cell`, and the
reply echoes the pixel only. Read the resulting cell with `battle.selection`. A host with no
primary window refuses `WrongState` naming it.

Whether that pixel becomes a cell is `pick_hovered_cell`'s call, and it resolves `None` unless a
`WorldCamera` and a primary window are both there, the cursor sits off the UI and inside the
viewport, and the pixel lands on the grid.
[`crates/gdtf_battle_input/tests/picking/resolve.rs`](../../crates/gdtf_battle_input/tests/picking/resolve.rs)
asserts the projection and these refusals: no cursor position, an off-grid pixel, no
`WorldCamera`, and no primary window. That last one resolves a cell first and then despawns the
window, so it also catches a pick that holds the cell it last resolved instead of dropping it.
[`crates/gdtf_battle_input/tests/picking/viewport.rs`](../../crates/gdtf_battle_input/tests/picking/viewport.rs)
asserts the rest, a cursor outside the viewport rect and a cursor over a HUD panel.
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
target, and refuses `WrongState` when nothing is focused. They all run in the same frame band
the real keyboard bridge runs in, so the screen has acted by the time the reply lands.

`input.click_cell` is the left click. It makes the named cell the hovered inspect target and
then runs the game's own decide-then-apply pair — `decide_left_click`, `decide_pin`,
`apply_left_click`, `apply_pin` — so one click may select a ganger, pin a move target, confirm
the move or fire, and the decision is the game's rather than the caller's. It is not a side
door into the sim: an act it decides goes onto the same `PendingActIntent` bus every act does,
and it needs the same `Running` plus `Caught` state every classic act needs. It answers
`ClickReply { decision, act }`: `decision` is which of `Fire`, `Select`, `SetMoveTarget`,
`Move`, `NoOp` and `Clear` the game chose, and `act` is the `ActReply` that decision earned —
the window, or the sim's typed refusal. Only `Fire` and `Move` push an act; `Select`,
`SetMoveTarget`, `NoOp` and `Clear` push nothing, so their `act` is absent. A shot the game
would not take is never decided as `Fire` in the first place — the click falls to another
decision rather than answering `FireRefused`. It carries no turn-owner check of its own: that
one is `act.end_turn`'s alone.

The view and battle controls all need a running battle with its sim state loaded, and
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
the offset taken. `battle.set_fire_mode` sets the mode on the same weapon the action bar's mode
panel sets it on, through the same lookup — the weapon the shooter fires, which is the mounted
one while they man an emplacement and the gun in their hands otherwise. Its write is the last of
them, landing after a panel press on that same frame and before the move-target reset. So the
mode in the reply is the mode `act.fire` and `battle.selection` read on that frame — both order
after `FireModeSystems::Write` — and the one every other reader sees after it. A press cannot
put the weapon back on single once the reply has gone out. It refuses `MissingModel` with a
note saying which precondition is missing when nothing is selected, the shooter has no weapon
to fire, or that weapon does not offer the mode.

Args, replies and published shapes are all **RON**. `arguments` is a string of compact RON
shaped by that command's own `schemas.arguments`; a command that takes none is `"()"`. The
MCP envelope itself stays JSON-RPC — the RON is opaque text riding inside it.

`commands` reads the catalogue from the RUNNING host, so its `availability` column is the
live answer rather than a static claim. Each of these can go wrong, and each tells you how to
fix it in one round trip: `Unknown` lists every name the host does offer, `BadArguments`
carries the schema your body was read against, `Unavailable` names the precondition that is
missing. `BadArguments` names the offending field in its `detail`, which is what
`deny_unknown_fields` really buys. A handler answers it too, through
`CommandResponder::bad_arguments`, for a body that decoded but named a value the command
cannot take: a stat outside its range, an index past the end of a list.

`run`'s two riders both work, on either host. `await_ready` holds a call its command refuses
right now and tests admission again every frame until the command admits; when the seconds run
out the answer is the command's own `Unavailable` refusal, the one the last test produced, never a
rider refusal and never `Timeout`. Nothing clamps the budget: a waiting call holds the
channel exactly as a parked `wait` does, and a budget past the socket's three-minute wait
for a reply (`DEFAULT_REPLY_TIMEOUT` in `crates/gdtf_qa_protocol/src/timeouts.rs`) answers
`Timeout` from the connection instead. A name the host does not know still answers `Unknown`
at once. `capture` runs the command first and appends one PNG attachment to the reply it
produced; an outcome that is not `Ran` comes straight back with no shot taken, and a shot
that never lands answers `Timeout` rather than the reply without its PNG.

## The editor host

The editor publishes thirty-eight commands, all in `EDITOR_COMMANDS`
([`crates/gdtf_content_editor/src/net_qa/commands/set.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/set.rs)).
Thirty-five are `Immediate`. Three are `Deferred`, so a client reads the timing to
know the reply can land on a later frame than the one that claimed the call: the two the
editor publishes under the game host's own spellings, `capture.screenshot` and `wait`, plus
`editor.delete_record`, whose in-use check settles across frames. A capture always
does, because the pipeline settles before it reads the pixels back. A `wait` whose condition
already holds is answered on the frame it was claimed in.

Five commands answer at every point in the lifecycle: the three reads `editor.phase`,
`editor.last_save` and `editor.validation`, plus `capture.screenshot` and `wait`, which need
neither the authoring scene nor a tab. Five more reads need the authoring scene:
`editor.families`, `editor.session`, `editor.draft`, `editor.map` and `editor.weighting`.
`editor.draft` also needs a form tab open, `editor.map` needs the Prefab tab, and
`editor.weighting` needs the Injury tab. All twenty-eight writes need the authoring scene too.
During the editor's Load pass every scene-scoped command answers `Unavailable { code: WrongState }`. `EditorMode`,
`MapEditorSession` and every draft are state-scoped to `EditorState::Editing` by the
`init_state_scoped_resource` calls in `MapEditorPlugin::build`
([`crates/gdtf_content_editor/src/plugin.rs`](../../crates/gdtf_content_editor/src/plugin.rs)),
so during Load none of them is in the world. The content registries are not state-scoped.
They arrive one at a time as the editor loads them, and Editing starts only once every one
of them is present (`transition_to_editing` and `GateResources::all_present` in
[`crates/gdtf_content_editor/src/load/transition.rs`](../../crates/gdtf_content_editor/src/load/transition.rs)),
which is why `editor.families` waits for Editing instead of answering a half-loaded set.

| Command | Availability | What it does |
| --- | --- | --- |
| `editor.phase` | always | The lifecycle phase, the mode tab open now, and every tab in tab-bar order. [`commands/read/editor_phase.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/read/editor_phase.rs) |
| `editor.last_save` | always | What the newest save per mode did: the file it wrote, or the fault it reported. [`commands/read/last_save.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/read/last_save.rs) |
| `editor.validation` | always | The content integrity report: every finding, plus whether the reference checks have run and whether the report has been published. [`commands/read/validation.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/read/validation.rs) |
| `editor.families` | Editing | The registry keys an author can pick, family by family, each with the label the editor's own picker shows. Its `family` argument names a form with the same names `editor.set_mode`'s `mode` takes, and `Prefab` answers `BadArguments`, because that tab is the map canvas and carries no content family. [`commands/read/families/`](../../crates/gdtf_content_editor/src/net_qa/commands/read/families) |
| `editor.session` | Editing | The theme and its default floor, the grid extent, the selected paint tile, the facing a paint turns it to, the storey being edited, and the view: draw mode, isolation, zoom and pan. [`commands/read/session.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/read/session.rs) |
| `editor.draft` | Editing + any form tab | The open form's draft as the RON text its save would write. [`commands/read/draft/`](../../crates/gdtf_content_editor/src/net_qa/commands/read/draft) |
| `editor.set_mode` | Editing | Opens a mode tab, writing the same `EditorMode` resource the tab bar and the number hotkeys write. [`commands/write/set_mode.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/set_mode.rs) |
| `editor.new` | Editing | Replaces a mode's draft with that form's own blank-draft constructor. [`commands/write/blank/`](../../crates/gdtf_content_editor/src/net_qa/commands/write/blank) |
| `editor.load_theme` | Editing + the Theme tab | Selects a theme in the authoring session and loads its def into the Theme draft, the same pair the Theme tab's own sync runs when the top bar picks a theme. The Theme form draws no load picker of its own. The load also takes the file the def was read from, so a later `editor.save` rewrites that file. [`commands/write/load/commands/theme.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/load/commands/theme.rs) |
| `editor.load_gang` | Editing + the Gang tab | Loads a gang roster by key into the Gang draft, through that draft's own `load_gang` method. [`commands/write/load/commands/gang.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/load/commands/gang.rs) |
| `editor.load_armor` | Editing + the Armor tab | Loads an armor by key into the Armor draft, through that draft's own `load_armor` method. [`commands/write/load/commands/armor.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/load/commands/armor.rs) |
| `editor.load_injury` | Editing + the Injury tab | Loads an injury by key into the Injury draft, through that draft's own `load_injury` method. [`commands/write/load/commands/injury.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/load/commands/injury.rs) |
| `editor.load_sprite` | Editing + the Sprite tab | Loads a sprite def by key into the Sprite draft, through that draft's own `load_sprite` method. [`commands/write/load/commands/sprite.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/load/commands/sprite.rs) |
| `editor.load_attachment` | Editing + the Attachment tab | Loads an attachment by key into the Attachment draft, through that draft's own `load_attachment` method. [`commands/write/load/commands/attachment.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/load/commands/attachment.rs) |
| `editor.load_weapon` | Editing + the Weapon tab | Loads a ranged weapon by key into the Weapon draft, through that draft's own `load_weapon` method. [`commands/write/load/commands/weapon.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/load/commands/weapon.rs) |
| `editor.load_melee_weapon` | Editing + the MeleeWeapon tab | Loads a melee weapon by key into the MeleeWeapon draft, through that draft's own `load_melee_weapon` method. [`commands/write/load/commands/melee_weapon.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/load/commands/melee_weapon.rs) |
| `editor.load_field` | Editing + the Field tab | Loads a field def by key into the Field draft, through that draft's own `load_field` method. [`commands/write/load/commands/field.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/load/commands/field.rs) |
| `editor.load_terrain` | Editing + the Terrain tab | Loads a terrain def into the Terrain draft, through that draft's own `load_from_def` method. The key is the hyphenated UUID text `editor.families` answers. A key that is not UUID text, and a key the registry does not hold, come back as `NoSuchKey` carrying that registry's own keys, sorted, with the draft left alone. The load also takes the file the def was read from, so a later `editor.save` rewrites that file. [`commands/write/load/commands/terrain.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/load/commands/terrain.rs) |
| `editor.load_prefab` | Editing + the Prefab tab | Opens an authored prefab onto the canvas the way the Prefab tab's own picker does: its grid size and theme onto the session, its painted cells onto the map, and the edit storey clamped into the loaded extent. The arguments are the prefab name and its key, which is the theme's UUID text, the grid size and the spawn role. A name and key no prefab sits under answers `NoSuchPrefab` and writes nothing. [`commands/write/load/commands/prefab.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/load/commands/prefab.rs) |
| `editor.save` | Editing | Writes a mode's draft through the same `write_*_in` its save button calls. [`commands/write/save/`](../../crates/gdtf_content_editor/src/net_qa/commands/write/save) |
| `editor.set_field` | Editing + any form tab | Writes one single-value field of the open form's draft, the way that form's own widget writes it. [`commands/write/set_field/`](../../crates/gdtf_content_editor/src/net_qa/commands/write/set_field) |
| `editor.list_op` | Editing + any form tab | Edits one list-valued field of the open form's draft through that form's own setter. [`commands/write/list_op/`](../../crates/gdtf_content_editor/src/net_qa/commands/write/list_op) |
| `editor.select_theme` | Editing | Selects a theme in the authoring session, taking that theme's own default floor with it, the way the top bar's picker does. [`commands/write/select_theme.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/select_theme.rs) |
| `editor.toggle_terrain` | Editing + the Theme tab | Adds or removes one terrain in the Theme draft's list, the way ticking its row in the terrain library does. [`commands/write/toggle_terrain.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/toggle_terrain.rs) |
| `editor.set_default_floor` | Editing + the Theme tab | Sets the Theme draft's default floor from the slabs the draft already holds. [`commands/write/set_default_floor.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/set_default_floor.rs) |
| `editor.map` | Editing + the Prefab tab | Every painted cell on one storey with its tile and facing, plus the grid extent. Nothing is filtered out, so a slot a shrink left outside the grid is still listed. [`commands/read/painted_map.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/read/painted_map.rs) |
| `editor.set_grid_size` | Editing + the Prefab tab | Sets the grid's width, height and storey count through the same commit the size fields call, clamping each span and re-clamping the edit storey. [`commands/write/set_grid_size.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/set_grid_size.rs) |
| `editor.select_tile` | Editing + the Prefab tab | Selects the tile the canvas paints, from the same rows the palette draws. [`commands/write/select_tile.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/select_tile.rs) |
| `editor.select_facing` | Editing + the Prefab tab | Selects the side the canvas turns the tile it paints to, the same choice the palette's facing row writes. Both paint entry points read it off the session. [`commands/write/select_facing.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/select_facing.rs) |
| `editor.set_level` | Editing + the Prefab tab | Jumps the storey the canvas paints on, the way clicking a row of the level rail does, clamping to the grid's extent. [`commands/write/set_level.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/set_level.rs) |
| `editor.paint` | Editing + the Prefab tab | Paints the selected tile in one cell of the storey being edited, through the same placement and connector pairing a canvas click runs. [`commands/write/paint/`](../../crates/gdtf_content_editor/src/net_qa/commands/write/paint) |
| `editor.select_injury_tab` | Editing + the Injury tab | Opens the Injury def form or the Injury weighting table, writing the same `InjurySubTab` resource the sub-tab row writes. [`commands/write/select_injury_tab.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/select_injury_tab.rs) |
| `editor.select_weighting_table` | Editing + the Injury tab | Loads the weighting table for one injury category and damage context, the way the Injury tab's two selectors load it. All three buckets are replaced from the tables, so unsaved row edits are discarded. Tables absent answers `Unavailable { code: MissingModel }`. [`commands/write/select_weighting_table.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/select_weighting_table.rs) |
| `editor.weighting` | Editing + the Injury tab | The weighting table the Injury tab holds: its category and damage context, and each of its Minor, Major and Critical buckets in the order the draft holds them. [`commands/read/weighting.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/read/weighting.rs) |
| `editor.save_weighting` | Editing + the Injury tab | Writes the weighting draft through the same `write_weighting_in` its Save weighting button calls, under the QA assets root, and records the outcome the way the button does. [`commands/write/save_weighting.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/save_weighting.rs) |
| `editor.delete_record` | Editing + the screen that offers the entry | Deletes one authored record, naming the delete entry's `FindingFamily` label, the record's `ContentMemberKey`, an optional replacement `ContentMemberKey`, and a `cancel` argument. Eleven families are deletable: prefab, injury weighting table, armor, melee weapon, attachment, injury, field, terrain def, theme, gang and ranged weapon. The in-use check runs first. Each referring family then resolves its own reference, either by dropping it or by taking the replacement: a gang member goes back to no armor, no melee weapon and no ranged weapon, a ranged or melee weapon back without the attachment, a weighting table without the injury's rows, and the situation, the terrain defs and the ranged weapons without the field, while a theme's floor and palette, a prefab's theme and placements, the situation's piece, theme and gang keys, another def's `leaves_behind` and an emplacement's `mounted_weapon` take the replacement. Naming a replacement confirms the delete. Omitting one on a record with a required referrer answers `Refused(InUse)` with the unresolved referring records, and the record goes back into its registry. A gang replacement whose roster does not hold a member name the situation gives the deleted gang answers `Refused(ReplacementLacks)` naming that member, with nothing written. `cancel` answers `Cancelled` with nothing written and nothing removed, and `cancel` together with a replacement key answers `BadArguments` naming both. The check runs a second time over the rewritten content. Nothing is rewritten for a prefab or a weighting table, and the melee weapon delete rewrites nothing while the `fists` default is absent, because a member left holding no melee weapon resolves to it. A family this build cannot delete comes back `Refused(NoEntry)`, and a family the open mode and sub-tab does not offer answers `Unavailable { code: WrongState }`. `Deferred`. [`commands/write/delete_record.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/write/delete_record.rs) |
| `capture.screenshot` | always | Writes a PNG of what the editor is showing and attaches it to the reply. `Deferred`. [`commands/capture/screenshot.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/capture/screenshot.rs) |
| `wait` | always | Holds the reply until `ChecksComplete` or `RegistryRearmed` comes true. `Deferred`. [`commands/wait/`](../../crates/gdtf_content_editor/src/net_qa/commands/wait) |

`editor.set_field` and `editor.list_op` need more than the authoring scene. Both write the
open form's own draft, so they need a form tab open and that tab's draft resource in the
world. The Prefab tab is the map canvas and holds no draft, so it answers
`Unavailable { code: WrongState }`, and so does a form tab whose draft has left the world
(`only_in_a_form_mode_with_its_draft` in
[`commands/availability.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/availability.rs),
reading the draft-presence fact `EditorFactsParam::sample` takes for the mode that is open).
`editor.toggle_terrain` and
`editor.set_default_floor` are scoped the same way to the Theme tab, because the terrain
library and the default-floor picker are both drawn only in the Theme arm of the central
and right panels, and both answer `Unavailable { code: WrongState }` on any other tab. The
seven prefab-canvas commands are scoped to the Prefab tab for the same reason: the palette,
the size fields, the level rail, the viewport and the open picker are drawn only in the
Prefab arm, and the painted map is that canvas's own model. The Prefab tab carries `EditorMode`'s own default,
so a fresh process reaches them with no `editor.set_mode` first.
`editor.select_injury_tab` is scoped to the Injury tab, because the sub-tab row it writes is
drawn only in the Injury arm of the central panel. So are the three weighting-table
commands, `editor.select_weighting_table`, `editor.weighting` and `editor.save_weighting`,
which read and write the weighting draft the Tables sub-tab draws. The weighting rows
`editor.set_field` and `editor.list_op` reach are scoped by their own arms instead: a
`Weighting(RowInjury(…))`, `Weighting(RowWeight(…))` or `WeightingBucket` name on any tab but
Injury answers `Unavailable { code: WrongState }` with the note that names the open tab.
The Field tab has one gate of its own, and it is a per-form refusal rather than an
availability rule: while `FieldDraft::autoload_pending` is true the form's own sync has not
run its first-frame seed yet, so every Field write answers
`Unavailable { code: WrongState }` with a note naming the autoload, and a QA write can never
be seeded over on the next egui frame. `editor.new` with `(mode: Field)` settles it.
`editor.select_theme` needs no particular tab, because the top bar draws its picker on every
one. The phase is checked first
([`commands/availability.rs`](../../crates/gdtf_content_editor/src/net_qa/commands/availability.rs)),
so a call during Load carries the phase note and not the tab note.

A field rides under the form that owns it, and its value rides on its own variant.
`Terrain(Kind(Emplacement))`, `Armor(Floor(part: Head, value: 12))`,
`Sprite(Frame(index: 1, source: File("a.png")))`, `Toggle(EntrySide(East))` and `MoveUp(2)` are
five of them. So an unknown form, field or list name is a decode failure answering
`BadArguments` with the schema. Neither command adds a protocol variant. What the handler
answers itself, in the order it checks:

1. The open mode's draft resource left the world between the availability check and the
   handler: `Unavailable { code: WrongState }`, with a note naming that mode's draft. A
   missing draft is host state, so the handler names the code the catalogue already names.
2. Any `editor.list_op` while the Armor tab is open: `BadArguments`. The Armor form draws six
   fixed pieces and no list, so no list name could be right there.
3. A field or list arm belonging to another form than the open tab:
   `Unavailable { code: WrongState }`, with a note naming the tab that is open.
4. An operation the named list does not draw: `BadArguments`. `TerrainTags`, `EntrySides` and
   `FieldImmuneArmorTypes` are rows of tick boxes, so each takes `Toggle` alone and refuses
   `Add`, `Remove`, `SetAt`, `MoveUp` and `MoveDown`, and a toggle carrying a member one of
   the other tick-box rows draws is refused too.
   Every other list refuses `Toggle`: the attachment effects, the sprite frames, the injury
   effects, the Terrain on-death effects, all three Melee Weapon lists and all four Weapon
   lists. Every list but the sprite frames and the two on-death lists refuses a reorder. The
   Gang member list and the three weighting buckets take `Add` and `Remove` alone, so each
   refuses `Toggle`, `SetAt`, `MoveUp` and `MoveDown`. When a list reads its own options from
   a registry and that registry is absent, the answer is `Unavailable { code: MissingModel }`
   instead.
5. A closed gate: `Unavailable { code: WrongState }`, with a note naming the gate.
   `editor.list_op` refuses a `Toggle` on the entry sides while the draft's kind is not
   `Emplacement`, because `TerrainDraft::set_entry_sides` commits only at that kind and the
   write would otherwise do nothing. The Sprite fps, every frame operation and every
   per-frame source refuse the same way while `SpriteDraft::is_animated` is false. The Weapon
   form has two gates of the same kind. The three DOT rows refuse while its `dot` tick box is
   off. An on-death payload row belonging to the other variant refuses as well, so
   `OnDeathField` refuses while the effect at that index is `Explode`, and `OnDeathHitType`,
   `OnDeathDamage` and `OnDeathDamageType` refuse while it is `LeaveField`. The Terrain form
   draws that second gate over its own on-death list, worded over the Terrain draft. No
   refused payload write turns a profile on, seeds an effect, or swaps a variant as a side
   effect. The Field form's one gate is read ahead of item 4 rather than after it: while
   `FieldDraft::autoload_pending` is true, every Field field and every operation on the immune
   list refuses here, whatever the write names, because the form's own sync would seed the
   first registry entry over it. A `Remove` on the injury effects, the melee fight modes or
   the weapon fire modes while one entry remains refuses here too, with a note naming the
   minimum, because it is the condition that greys out the form's own Remove button. The
   sprite frames answer that case as `BadArguments` instead, under item 6.
6. A stat outside its range, an index past the end of a list, a `Remove` on a sprite frame
   list holding one frame, a `MoveUp` at index 0 and a `MoveDown` at the last index:
   `BadArguments`. A Gang member key answers the same way when the registry the combo reads
   does not hold it, weapons against `WeaponRegistry`, melee weapons against
   `MeleeWeaponRegistry` and armor against `ArmorRegistry`, and the detail names the key. A
   registry that is absent, or present and empty, offers the combo no row at all, so that one
   answers `Unavailable { code: MissingModel }`. Clearing a member's melee weapon with
   `Gang(MemberMeleeWeapon(index: 0, key: None))` reads no registry, so it lands with
   `MeleeWeaponRegistry` out of the world. The same holds for `Gang(MemberArmor(index: 0,
   key: None))` and `Gang(MemberWeapon(index: 0, key: None))`: a `key: None` clear reads no
   registry, so each lands with `ArmorRegistry` or `WeaponRegistry` out of the world. A
   fitted attachment key the `AttachmentRegistry`
   holds no spec for answers `BadArguments` too, on the Weapon lists as on the Melee Weapon
   ones.

Every write that lands answers `Ran`, and the reply carries the mode, the field or list the
write named, and the value as the draft stores it. Where the form's own input clamps a drag rather
than refusing it, so does the write, and the reply reads the stored value back. `SpriteDraft::set_anchor` holds each
axis inside the sheet rect, so an anchor beyond the rect's width answers `Ran` at the edge.
The Terrain `Hp`, `ArmorProtection` and `ArmorHardness` writes hold the value inside
`TerrainDraft::HP_RANGE` and `TerrainDraft::ARMOR_RANGE`, the Melee Weapon `Reach` write
holds it at one or above, and the Weapon `DotTurns` write stores a zero as one.

### The fields and lists each form offers

Every field goes inside its form's own arm, which the middle column names. The Armor row's
`Floor` is `Armor(Floor(part: Head, value: 12))` on the wire, and the Injury weighting row's
`RowWeight` is `Weighting(RowWeight(bucket: Minor, index: 0, weight: 4))`.

| Form | Wire arm | Fields |
| --- | --- | --- |
| Armor | `Armor` | `Name`; per `BodyPart`, `Floor`, `Protection`, `Hardness` (`ArmorDraft::STAT_RANGE`, `0..=100`), `Integrity` (`ArmorDraft::INTEGRITY_RANGE`, `0..=1000`) and `Type` (one of `ArmorType::ALL`) |
| Sprite | `Sprite` | `Name`; `BaseSource`; `AnchorX` and `AnchorY` (clamped to the sheet rect); `Animated`; and behind that gate `Fps` (`SpriteDraft::FPS_RANGE`), `Frame`; `FacingOverride`, set to a source or cleared with `None` |
| Attachment | `Attachment` | `Name`; `DisplayName`; `Slot` (one of `AttachmentSlot::ALL`); `Effect`, variant and payload together |
| Terrain | `Terrain` | `Kind` (Wall, Cover, Slab, Emplacement); `DisplayName`; `Hp`, which writes both the cover and the slab field, clamped to `TerrainDraft::HP_RANGE`; `ArmorProtection` and `ArmorHardness`, clamped to `TerrainDraft::ARMOR_RANGE`; `HeightBand`, on a kind whose `has_height_band` is true; `Footfall`, on a kind whose `offers_footfall` is true; `MountedWeapon`, a `WeaponRegistry` key or `None`, on an `Emplacement` kind; `BlocksPathing` and `BlocksLos`, each set or cleared with `None`; `LeavesBehind`, one of `Nothing`, `Piece("…")` naming a `TerrainDefRegistry` key, or `Sprite("…")` naming a `SpriteDefRegistry` key; `View`, carrying the view and the sprite key that draws it, refused with `WrongState` for a view the open draft's kind and tags do not owe; per on-death `index`, `OnDeathVariant` (Explode or LeaveField), then `OnDeathHitType`, `OnDeathDamage` and `OnDeathDamageType` while the effect at that index is `Explode`, and `OnDeathField` while it is `LeaveField` |
| Theme | `Theme` | `Name`, the display name its name box writes. Its default floor is written with `editor.set_default_floor` |
| Injury | `Injury` | `Key`; `Name`; `Category` (one of `InjuryCategory::ALL`); `Severity` (Minor, Major or Critical, so `None` and `Fatal` fail to decode); `PopupText`, `LogText`, `InspectText`; `Effect`, an index with that effect's variant and payload |
| Injury weighting | `Weighting` | `RowInjury` and `RowWeight`, each naming a `bucket` (Minor, Major or Critical, so `None` and `Fatal` fail to decode) and a row `index`. The key must be one the `InjuryRegistry` holds, because that is all the row's combo offers, so a key it does not hold answers `BadArguments` naming the key, and an absent registry answers `MissingModel`. Which table the rows belong to is `editor.select_weighting_table`, not a field |
| Melee Weapon | `MeleeWeapon` | `Name`; `Damage`, `Punch`, `Shred`, `DamageType` (one of `DamageType::ALL`), `FatalBias`, `Handedness`; `Reach`, clamped at 1; `Shove` |
| Gang | `Gang` | `Name`; per member index, `MemberName`; `MemberAttribute`, naming one of `Speed`, `Aim`, `Strength`, `Toughness`, `Reflexes`, `Cool`, `Grit` and `Luck`, with a value inside `GangDraft::ATTRIBUTE_RANGE` (`0.0..=100.0`); `MemberWeapon`, a `WeaponRegistry` key or `None` clearing it; `MemberArmor`, an `ArmorRegistry` key or `None` clearing it; `MemberMeleeWeapon`, a `MeleeWeaponRegistry` key or `None` for the fists default |
| Field | `Field` | `Name`, the save stem; `Damage`, the flat HP drained per tick; `DamageType` (one of `DamageType::ALL`); `Duration`, `Permanent` or `Turns(n)` where `n` is at least 1, and `Turns(0)` answers `BadArguments` rather than being clamped |
| Weapon | `Weapon` | `Name`; `BaseSpread`, `Accuracy`, `Kickback`; `Damage`, `Punch`, `Shred`, `DamageType` (one of `DamageType::ALL`), `FatalBias`, `Handedness`; `Trajectory` (Straight or Arc), `Stable`, `Shove`; `MagazineSize`, `MagazineReloadTu`; `Dot`, and behind that tick box `DotDamage`, `DotTurns` (zero is stored as 1) and `DotDamageType`; per on-death `index`, `OnDeathVariant` (Explode or LeaveField), then `OnDeathHitType`, `OnDeathDamage` and `OnDeathDamageType` while the effect at that index is `Explode`, and `OnDeathField` while it is `LeaveField`. There is no `accepts` arm, because the form draws no control for it |

| Form | List | Operations |
| --- | --- | --- |
| Armor | none | The six pieces are fixed, so every `editor.list_op` answers `BadArguments` |
| Sprite | `SpriteFrames` | `Add` (copies the last frame, else the base source), `Remove`, `MoveUp`, `MoveDown`. Animation must be on, and the list keeps at least one frame |
| Attachment | `AttachmentEffects` | `Add` (seeds the form's own default effect), `Remove`. An empty list is legal and means cosmetic |
| Terrain | `EntrySides` | `Toggle(EntrySide(..))`, on an `Emplacement` kind |
| Terrain | `TerrainTags` | `Toggle(TerrainTag(..))` |
| Terrain | `TerrainOnDeathEffects` | `Add` (seeds `weapon_form::explode_template`, the same blank explode the Weapon form's Add seeds), `Remove`, `MoveUp`, `MoveDown`. A row is rewritten through the `Terrain(OnDeath…(index: n, …))` field arms, so the list refuses `Toggle` and `SetAt`. An empty list is legal and means the piece fans nothing |
| Theme | none | The tab draws no list of its own, so every `editor.list_op` answers `Unavailable { code: WrongState }`. Its terrain library is written with `editor.toggle_terrain` |
| Injury | `InjuryEffects` | `Add` (seeds the form's own default effect), `Remove`. The list keeps at least one effect |
| Injury weighting | `WeightingBucket(Minor \| Major \| Critical)` | `Add` (seeds the first `InjuryRegistry` key by name, at weight 1), `Remove`. A row is rewritten through the `Weighting(RowInjury(…))` and `Weighting(RowWeight(…))` field arms, so the list refuses `Toggle`, `SetAt` and both reorders. An absent or empty `InjuryRegistry` answers `MissingModel` on `Add`, which is the condition that greys out the form's own Add button. An empty bucket is legal |
| Melee Weapon | `MeleeWeaponFightModes` | `Add` (seeds the form's own structural swing), `Remove`, `SetAt`. The list keeps at least one mode |
| Melee Weapon | `MeleeWeaponSlots` | `Add` (seeds `WeaponSlots::DEFAULT_DECLARATION`), `Remove`, `SetAt`. An empty list is legal |
| Melee Weapon | `MeleeWeaponAttachments` | `Add` (seeds the registry's first key by name), `Remove`, `SetAt` naming a key the registry holds. An absent `AttachmentRegistry` answers `MissingModel` on `Add` and on `SetAt`. An empty one answers `MissingModel` on `Add`, because there is no key to seed, and `BadArguments` on `SetAt`, because the key it names is not one the registry holds |
| Field | `FieldImmuneArmorTypes` | `Toggle(ImmuneArmorType(..))` alone. The form draws one tick box per `ArmorType::ALL`, so the list has no append, no position and no order, and every other operation answers `BadArguments`. An empty list is legal and means the field drains everyone |
| Gang | `GangMembers` | `Add` (appends the form's own default member), `Remove`. No minimum, so an empty list is legal, and no reorder |
| Weapon | `WeaponFireModes` | `Add` (appends through `WeaponDraft::add_fire_mode`), `Remove`, `SetAt` rewriting all five fields the row draws, `hit_type` included. The list keeps at least one mode |
| Weapon | `WeaponSlots` | `Add` (seeds `WeaponSlots::DEFAULT_DECLARATION`), `Remove`, `SetAt`. An empty list is legal |
| Weapon | `WeaponAttachments` | `Add` (seeds the registry's first key by name), `Remove`, `SetAt` naming a key the registry holds. An absent `AttachmentRegistry` answers `MissingModel` on `Add` and on `SetAt`. An empty one answers `MissingModel` on `Add`, because there is no key to seed, and `BadArguments` on `SetAt`, because the key it names is not one the registry holds |
| Weapon | `WeaponOnDeathEffects` | `Add` (seeds `weapon_form::explode_template`), `Remove`, `MoveUp`, `MoveDown`. A row is rewritten through the `Weapon(OnDeath…(index: n, …))` field arms, so the list refuses `Toggle` and `SetAt`. An empty list is legal and means the weapon fans nothing |

Reorder exists for the sprite frames and the two on-death effect lists.

A mode a command does not handle is a **typed outcome inside a successful reply**, never
`Unavailable` — that is reserved for host state. `editor.new` refuses Terrain and Prefab
because neither form draws a New button, and refuses Theme because the theme form's own
sync reloads the session theme's def over any draft whose key differs, so a blank Theme
draft would not survive a frame. A key no registry holds answers `NoSuchKey` with the keys
it does hold, and leaves the draft untouched.

The three theme helpers take one key and no mode, so they have no typed outcome to carry a
miss. All three answer `Unavailable { code: MissingModel }` for a key that is not UUID text.
`editor.select_theme` and `editor.toggle_terrain` answer it again for UUID text their
registry does not hold, leaving the session or the draft as it was.
`editor.set_default_floor` refuses on a narrower test, so it answers
`Unavailable { code: WrongState }` for every key outside `slab_floor_candidates`
([`theme_form/resolve.rs`](../../crates/gdtf_content_editor/src/theme_form/resolve.rs)), the
list the form's own picker offers. That one code covers all three misses: a terrain the
draft does not hold, a terrain the draft holds that is not a slab, and UUID text no registry
holds. `ThemeDraft::set_default_floor` writes nothing when the draft does not hold the key,
so answering `Ran` there would report a write that never happened. A non-slab key the draft
does hold it would write, and refusing that one keeps the command from authoring a floor the
form's picker would never offer.
`editor.toggle_terrain`'s reply says which way the tick went, `Added` or `Removed`, and
carries the draft's default floor afterwards, because removing the terrain that was the
default floor clears it.

The prefab canvas commands answer their own misses the same way. `editor.select_tile` and
`editor.paint` carry a typed refusal in the `Ran` body: which of the palette's two
conditions a key failed, and whether a paint had no tile selected. `editor.select_tile`
still answers `Unavailable { code: MissingModel }` for a key that is not UUID text, as the
theme helpers do. An illegal paint is a `Ran` reply whose verdict is
`Illegal(OutOfBounds)` or `Illegal(SlabSealsLadder)`, because the verdict is the reply's
content, not a fact about the host. That verdict is read before the write, so a placement
that clears a slab above names the slot it is about to empty. The reply also carries what
the connector pairing pass did. `PairPlaced` names the tile the pass placed one storey up,
the same tile that was painted, and the slot it landed in, so the next `editor.map` read
holds no cell the caller cannot account for.

`editor.save` writes under `EditorQaAssetsRoot`
([`net_qa/assets_root.rs`](../../crates/gdtf_content_editor/src/net_qa/assets_root.rs)),
which defaults to the workspace `assets/` and falls back to a temp directory when the
marker search finds no workspace. Only Prefab takes a `name`, because only the prefab form
carries its own name field; a name on any other mode is refused rather than dropped.

On the Terrain and Theme tabs where the write lands depends on where the draft came from.
A draft filled by `editor.load_terrain` rewrites the file that def was read from, whatever
its display name and the session theme now say; a draft the tab minted lands in the session
theme's folder under a stem taken from the display name. A draft filled by
`editor.load_theme` rewrites its own file the same way, keeping the stem and folder it was
read under; a theme the tab minted lands in a folder under `content/terrain/` named for its
display name, under a stem taken from the same name. `editor.last_save` reports whichever
path the save wrote.

`MapEditorPlugin` seeds that root, not the QA channel, so it is in the world whether or not
the listener bound. `editor.save_weighting` and the Injury tab's own Save weighting button
both write under it through `write_weighting_in`, the one writer either path has.

`editor.last_save` reads the same `LastSaveRecord`
([`crates/gdtf_content_editor/src/save_record/`](../../crates/gdtf_content_editor/src/save_record))
that all twelve of the editor's own save buttons write, so a QA-driven save and a
button-driven save are indistinguishable to it. The record is keyed by mode and is not
state-scoped, so it survives a Load ↔ Editing round trip.

`editor.validation` answers from the moment the process boots, because
`ContentIntegrityReport` is inserted at plugin build. The report starts empty, so the reply
carries two more flags beside the findings: `checks_complete` says whether
`ContentChecksComplete` is in the world, and `published` says whether `ContentValidationDone`
is. Both start false, and the checks only run once every registry they read is loaded, so a
reply taken early in Load carries `checks_complete: false` and `published: false`. An empty
findings list on its own is not a clean bill of health. Each finding is one line, rendered
through the same `Display` the editor's own log prints.

`editor.families` sorts every family's entries by rendered key before replying, because each
registry is a `HashMap` underneath and its own order changes between runs. A `family`
argument narrows the reply to that family; without one, all ten answer. The argument takes
the same form names `editor.set_mode`'s `mode` takes, so one set of names picks a form
across the whole editor. The one name it will not answer is `Prefab`: that tab is the map
canvas and owns no registry, so it comes back as `BadArguments` with a detail saying so.
For the two UUID-keyed families the label is the def's display name, which is what the
editor's own theme picker shows, and a display name two or more terrain defs share carries
that def's key, the same row the Terrain load picker draws; for the eight name-keyed
families the label is the key. The Terrain load picker appends the def's key to a label two
or more defs share, so a duplicated display name is still one row per def.

`editor.draft` reads the active tab and takes no mode argument. The RON is the mode's own
`draft_to_*` conversion followed by `gdtf_assets::serialize_ron_pretty`, the two calls
`write_ron_pretty` makes, so the text is what `editor.save` writes to the file. The Terrain
tab has one difference. The read takes the key route the terrain form's own preview pane
takes, `TerrainDraft::uuid` falling back to the nil UUID, while `editor.save` calls
`TerrainDraft::ensure_uuid` and mints a key. So a terrain draft that has never been saved
reads back with a nil `key` and is saved under a fresh one. Every other field matches, and a
draft loaded by key already carries its own.

On the Prefab tab `editor.draft` answers `Unavailable { code: WrongState }` with a note
naming `editor.map`, because Prefab holds no draft. A draft that will not convert, such as
an Emplacement terrain with no mounted weapon, answers a successful reply carrying
`NotSavable` with the fault, never a refusal and never an empty string.

`capture.screenshot` is the game host's command under the game host's spelling, so a client
never branches on which host answered. Its argument and reply are the same shapes: one
optional `name` that becomes the file stem, and a `Ran` reply carrying the path plus a
`ReplyAttachment(Png, …)` naming it. A capture that never lands answers `Timeout`, and
nothing refuses it. The pipeline is the shared one in `crates/gdtf_screenshot`, registered
by the editor's own `serve`
([`net_qa/plugin.rs`](../../crates/gdtf_content_editor/src/net_qa/plugin.rs)) as
`CapturePresentPlugin`, `WindowCapturePlugin` and `CapturePipelinePlugin<EditorShotResponder>`
together. Editor shots land in `target/qa_screenshots_editor` rather than the game's
`target/qa_screenshots`, so the two processes cannot write over each other when both are up.
`run`'s `capture` rider writes there too: `register_riders`
([`dispatch/register.rs`](../../crates/gdtf_qa_command/src/dispatch/register.rs)) adds
`CapturePipelinePlugin<CaptureTicket>` and the `drive_rider_captures` drain that empties it
beside the holds themselves, so the host that parks a held reply is the host that takes its shot.

`wait` holds its reply until one named condition becomes true, and it takes the game's
two-minute budget, which expires well inside the socket's own 180-second wait for a reply.
The editor has two conditions. `ChecksComplete` comes true once `ContentChecksComplete` is
in the world, which is the same marker `editor.validation` reports. `RegistryRearmed` names
one of ten content families — Weapon, MeleeWeapon, Armor, Gang, Terrain, Theme, Injury,
Sprite, Attachment and Field, spelled as the mode tabs are — and comes true when that
family's registry changes after the call was parked. Validation watches all ten
([`validate/rearm.rs`](../../crates/gdtf_content_editor/src/validate/rearm.rs)), so a rearm
of the validation pass is what a wait on one of them sees. Validation also watches
`PrefabRegistry` and the loaded situation, which no wait condition names. A condition that never
comes true answers `Timeout` and leaves the connection open, never a refusal and never an
"unsatisfied" reply.

## Why the shape is what it is

**A host publishes ONE list of typed commands, and the wire carries any command in
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
- The wire surface is exactly these request variants (`Catalogue`, `Run`) and these response
  variants (`Catalogue`, `Outcome`). A command is *data* inside them, so adding one moves
  no protocol version and adds no variant.
- The MCP courier exposes exactly these tools, `commands` and `run`. **Neither names a
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
