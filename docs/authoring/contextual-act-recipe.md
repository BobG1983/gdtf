# Adding a contextual act — the recipe

A **contextual act** is a target-carrying battlescape act: the panel offers a target
while one is in reach (Execute / Stabilize / Melee / Shove / Open Door / Enter
Emplacement / Exit Emplacement / Throw Grenade), a press — a mouse click on the button OR
the digit key bound to its visible slot — buffers that target, and the sim's
bespoke dispatch is the authoritative gate. Since the whole ritual is
descriptor + registrar shaped: **adding one act touches ONE new per-act module file per
crate layer plus ONE registration line per layer** (plus that act's bespoke sim
dispatch and its cost pair). There is no runtime descriptor table — registration is
compile-time generic (`add_contextual_act::<A>()`), mirroring `add_message::<M>`.

The drain invariant (Q5-approved wording): **per-act generic drains in one
explicitly-ordered SystemSet, same-frame semantics preserved.** A press queued this
update is drained to its `*Requested` this update, and the sim consumes it the same
frame (`ContextualPanelSystems::Press` → `ContextualActSystems::Drain` →
`dispatch_act_intents` → `SimSystems::Simulate`, all explicit edges). The QA command band
joins the same chain: `ActCommandSystems::Claim` is ordered before
`ContextualActSystems::Drain` in `commands/register.rs`, so a command's push is drained in
the frame it was claimed in, exactly as a press is.

## The stations, in order

### 1. Sim — the act itself (bespoke, load-bearing)

- **New message** in `crates/gdtf_battle_sim/src/acts/request/<family>.rs`: the
  `<Act>Requested` type (per-act message TYPES stay — no mega-enum), re-exported from
  `request/mod.rs`. Paired acts share a family file — `emplacement.rs` holds enter and exit.
- **New module** `crates/gdtf_battle_sim/src/acts/<act>/`: `dispatch.rs` holds the bespoke
  `dispatch_<act>` system, `cost.rs` holds `<act>_tu_cost(…) -> Tu` and
  `can_<act>(…) -> Can<Act>`. Both are pure — they take what they need, return a cost or a
  yes/no, and change nothing. The dispatch gates on `can_<act>` (adjacency / faction /
  state), charges exactly what `<act>_tu_cost` returned, and owns any act-specific ordering
  — these are deliberately NOT generic. A small act stays one file (`open_door.rs`,
  `reload.rs`, `enter_emplacement.rs`) with the same two pieces in it.
- **A pool that cannot cover the cost refuses the act.** `spend_tu` returns
  `Result<(), TuShortfall>` and never clamps, so a shortfall is a case the dispatch has to
  handle: skip the request, change nothing, spend nothing. The usual shape is to fold
  `can_spend_tu(tu, <act>_tu_cost(…))` into `can_<act>` itself — Open Door, Reload, Throw
  Grenade, Enter / Exit Emplacement, Execute, Stabilize and `can_set_stance` all do, so the
  dispatch gets one verdict rather than a second copy of the rule. Where the predicate is
  deliberately TU-blind (`can_shove`, `can_melee`), the dispatch calls `can_spend_tu` itself
  before touching anything, and the guarded `spend_tu` is the backstop. The one exception is
  `set_facing`: a turn is divisible, so it charges the whole 45deg steps the pool affords
  and lands partway.
- **Two requests for one target in the same frame charge once.** A dispatch reads its
  targets through a read-only query, and the toggle it writes is applied by a later
  system, so a second request in the same run sees the state the first one read and
  pays for a change that never happens. Open Door, Enter Emplacement and Exit
  Emplacement each keep a `PendingStates` map
  (`crates/gdtf_battle_sim/src/acts/pending_state.rs`) of what the run has already
  driven each target to. They hand that state to the predicate instead of the queried
  one and record the new state after the charge, so the repeat is refused by the same
  gate that refuses an open door or an occupied seat. Any act whose predicate reads the
  target's state needs the same; the drain invariant above is what puts two presses in
  one frame.
- **Sim owns the TU number.** Whoever needs it — dispatch, HUD, cursor preview, AI, a QA
  command — calls `<act>_tu_cost`; never a second copy of the math. Re-export the pair from
  `acts/mod.rs` so callers outside the sim can reach it. Execute and Stabilize charge that
  quote now — both predicates fold `can_spend_tu` in, and both dispatches debit the actor's
  pool by it — but only the cost half reaches `acts/mod.rs`; `can_execute` and
  `can_stabilize` stop at `acts/downed/mod.rs`, so an outside caller still goes through
  `acts::downed` for the verdict. Their verbs (`execute_downed` / `stabilize_downed`) also
  hand the cost back as a return value — a leftover, not a shape to copy.
- **Registration lines** in `crates/gdtf_battle_sim/src/acts/plugin/` (`SimActsPlugin`):
  `.add_message::<<Act>Requested>()` in `register_messages` and the dispatch system
  `.in_set(SimSystems::Simulate)` in `wire_acts` (with explicit `.before`/`.after`
  edges only where the act genuinely shares state — read the neighbouring comments).

### 2. Input — the descriptor (one file + one line)

- **New module** `crates/gdtf_battle_input/src/act_bus/contextual/<act>.rs`: a unit
  token type (`pub struct <Act>Act;`) implementing `ContextualAct` — `type Target`
  (what a press carries: an `Entity`, a `CellLevel`, or a small per-act domain enum
  like `MeleeTarget`), `type Requested`, and `fn request(actor, target)`. Wire the
  module + re-export in `act_bus/contextual/mod.rs`.
- **One line** in `crates/gdtf_battle_input/src/plugin/build.rs`
  (`register_contextual_acts`): `.add_contextual_act::<<Act>Act>()`. That wires the
  message buffer (idempotent with the sim's), the buffered
  `PendingContextualIntents<A>` queue, and the generic drain in the ONE
  `ContextualActSystems::Drain` set.
- The descriptor carries **no per-act keybind field** — but the act is no longer
  button-only. Since a keyboard DIGIT slot-key also activates it: digit N fires
  the Nth currently-visible contextual button unless that button is greyed out —
  the ranking counts greyed buttons, so the numbering never shifts (the binding is
  per-SLOT, not per-action,
  so it needs no descriptor field — the panel ranks the visible buttons and each per-act
  digit-press router resolves its key from that rank via `Keybinds::contextual_slot_key`).
  This reverses the Q8 "button-only" ruling; the keyboard slot-bindings coexist
  with mouse clicks, both feeding the one `PendingContextualIntents<A>` dispatch.

### 3. App — the panel button (one file + one line)

- **New module**
  `crates/gdtf_game/src/states/running/game/battlescape/contextual_panel/acts/<act>.rs`:
  the button marker (via `crate::support_item!`), the `ContextualPanelAct` impl
  (`type Marker`, `fn label()`, `const SLOT` — pick an UNUSED `PanelSlot`; slots order
  the column top-to-bottom and must stay unique), and the act's bespoke
  `offer_<act>` system (the scan deciding what to OFFER — advisory only; the sim
  re-gates). Wire the module in `contextual_panel/acts/mod.rs`.
- **Offer the act even when the actor cannot pay for it.** Geometry, faction, life and line
  of sight decide whether the button is on screen; affordability decides only whether it is
  pressable. Every scan takes its target test from the sim's predicate and writes no
  adjacency, faction, life or bleeding test of its own. Those whose predicate folds TU in
  — Execute, Stabilize, Open Door, Throw Grenade, Enter and Exit Emplacement — call it
  twice: once with the cost itself as the pool, which holds the TU term true so the
  remaining terms pick the target, then with the actor's real pool for the pressable bit.
  Melee and Shove have TU-blind predicates, so they call `can_melee` / `can_shove` for the
  target and `can_spend_tu` for the pool. Either way
  the answer goes to `ContextualOffer::with_pressable`, which greys the button through
  `gdtf_ui::DisabledButton` and reports itself on `battle.offers`. A greyed
  button ignores a click and its digit slot key. Melee also greys out when the actor wields
  no melee weapon, because there is no strike to price.
- **One line** in `contextual_panel/plugin.rs`:
  `.add_contextual_act_button::<<Act>Act, _>(acts::<act>::offer_<act>)`. That stamps
  the generic button spawn, visibility toggle, disabled sync, and press router over the
  descriptor.
- Never add per-act `Without<>` disjointness filters, per-marker visibility bundles, or
  per-marker press queries — the generic systems hold ONE query each, so none are
  needed (the N-squared filter wall must not come back).

### 4. AI arm, or documented why-not (named station — Q6, ruled)

Until lands, the enemy-AI act contract is **move / fire / end-turn, plus
reload-when-landed** — the brain (`crates/gdtf_battle_sim/src/ai/`) writes only `FireRequested` /
`MoveRequested` / `EndTurnRequested` today, and records the reload gap. Every
new act must EITHER add a brain arm that can emit its `*Requested`, OR record here (and
on the ticket) why the AI does not use it yet.

Why-not record for the existing contextual acts: they are player-affordance
surfaces pending the AI-acts expansion; none has a brain arm today.

### 5. QA command (one file + one line + one socket case)

- **New module**
  `crates/gdtf_game/src/dev/mcp/commands/act/contextual/<act>.rs`: a unit struct
  implementing `McpCommand` named `act.<act>`, with `type Args = NoArgs` unless the act names
  its own target (see the target bullet below),
  `type Parked = ContextualTicket`, `type Reply = ContextualReply`,
  `CommandTiming::Immediate` and `availability = running_and_caught`. It also implements
  `ContextualCommand`, which is where the act family is named — `type Act = <Act>` plus
  `const TARGET`, the mapper `battle.offers` already uses for that target kind (`ganger` /
  `door` / `emplacement` / `cell` / `melee` in `commands/read/battle_offers.rs`). The family
  is stated once and `TARGET` is typed against it, so a mapper from the wrong family will not
  compile. `register_handler` is then one call to `register_contextual::<Self>`, which puts
  the claim and settle systems in `ActCommandSystems::ContextualClaim` and
  `ActCommandSystems::Settle`
  — the bands already carry the edges to the panel's offer scan, to the contextual drain and
  to `SimSystems::Record`. Wire the module + re-export in `contextual/mod.rs`.
- **One line** in `crates/gdtf_game/src/dev/mcp/commands/set.rs` (`GAME_COMMANDS`), plus
  the matching name in `tests/mcp/command_exchange/names.rs`, the expected lists in
  `tests/mcp/command_set.rs` and `tests/mcp/commands.rs`, and the timing row in
  `tests/mcp/catalogue_acts.rs`.
- **The command may never act on a target the panel is not offering.** The panel offers
  exactly one target per family and no press carries a target of its own, so a command that
  acted on anything else would be a second code path doing something no player can do. Seven
  of the eight take `NoArgs` and act on whatever the offer holds. `act.melee` names its
  target — `type Args = ActMeleeArgs` carrying a `MeleeTargetNet` — because the melee scan
  falls back to a structure cell when no adjacent ganger is in sight, so a client that
  ignores a `NoLineOfSight` quote and swings anyway would otherwise smash a wall. Naming a
  target does not widen what the command can do: `ContextualCommand::target_refusal` compares
  it with the offer and answers `TargetMismatch` when the two differ. Its default body takes
  whatever is offered, which is what the other seven want. Either way the command reads
  `ContextualOffer<A>`, pushes `offer.target()` onto `PendingContextualIntents<A>`, and
  refuses `NoOffer` when the family is offering nothing. It evaluates no legality and no TU —
  the sim's `dispatch_<act>` is still the only gate.
- **One socket case** in `crates/gdtf_game/tests/mcp/contextual_acts/`, on the existing
  `battle_app_listening()` fixture: spawn the scenario relative to the selected shooter,
  call the command, assert the reply names the offered target. Assert on the WORLD, not the
  log, for any act `ActDeed` has no variant for — doors and emplacements log nothing, and an
  empty window there is correct.
- **Shape the scene so only this family is offered**, and move the generated map's own
  candidates out of reach first (`clear_doors_around`, `clear_enemies_around`, and the
  emplacement equivalent). That is what makes the case discriminating: a command wired to a
  neighbouring family — stabilize to execute, enter to exit — then answers `NoOffer` and the
  case goes red instead of silently acting on the wrong thing. Relying on the seeded map to
  hold no rival candidate is a map-shaped flake.
- **One line** in `docs/tooling/qa-commands.md`'s command list.

### 6. Tests + test-surface (as the act warrants)

- A headless end-to-end press test in the `crates/gdtf_game/tests/contextual_panel/`
  suite — one file per act, sharing `harness.rs`'s `battle_running_app()`: offer →
  press → assert the
  `*Requested` in the sim buffer (probe `.after(ContextualActSystems::Drain)`), plus
  the negative offer cases.
- To name the marker from the external test: 2 edits — add it to the panel's
  `test_support` submodule (`contextual_panel/mod.rs`) and to the crate-root ledger
  (`src/test_support.rs`), per the one-hop pattern.
- Input-layer drain coverage is generic and already pinned
  (`act_bus/contextual/test.rs`); sim dispatch gets its own bespoke tests.

## Worked reference

`Shove` (ported end-to-end in) is the reference walk: sim
`crates/gdtf_battle_sim/src/acts/shove/`, input `act_bus/contextual/shove.rs` +
`.add_contextual_act::<ShoveAct>()`, app `contextual_panel/acts/shove.rs` +
`.add_contextual_act_button::<ShoveAct, _>(acts::shove::offer_shove)`, QA
`commands/act/contextual/shove.rs` + its `GAME_COMMANDS` line, AI why-not
recorded above, and the press/offer/same-frame tests in
`crates/gdtf_game/tests/contextual_panel/shove.rs` plus the socket case in
`crates/gdtf_game/tests/mcp/contextual_acts/`.
