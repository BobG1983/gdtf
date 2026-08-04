# Adding a contextual act — the recipe

A **contextual act** is a target-carrying battlescape act: the panel offers a target
while one is in reach (Execute / Stabilize / Melee / Shove / Open Door / Enter
Emplacement / Exit Emplacement / Throw Grenade), a press — a mouse click on the button OR
the digit key bound to its visible slot — buffers that target, and the sim's
bespoke dispatch is the authoritative gate. Since the whole ritual is
descriptor + registrar shaped: **adding one act touches ONE new per-act module file per
crate layer plus ONE registration line per layer** (plus that act's bespoke sim
dispatch). There is no runtime descriptor table — registration is compile-time generic
(`add_contextual_act::<A>()`), mirroring `add_message::<M>`.

The drain invariant (Q5-approved wording): **per-act generic drains in one
explicitly-ordered SystemSet, same-frame semantics preserved.** A press queued this
update is drained to its `*Requested` this update, and the sim consumes it the same
frame (`ContextualPanelSystems::Press` → `ContextualActSystems::Drain` →
`dispatch_act_intents` → `SimSystems::Simulate`, all explicit edges).

## The stations, in order

### 1. Sim — the act itself (bespoke, load-bearing)

- **New module** `crates/gdtf_battle_sim/src/acts/<act>.rs`: the
  `<Act>Requested` message type (per-act message TYPES stay — no mega-enum) and the
  bespoke `dispatch_<act>` system. The dispatch owns the authoritative gate
  (adjacency / faction / state), the TU spend, and any act-specific ordering — these are
  deliberately NOT generic.
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
  the Nth currently-visible contextual button (the binding is per-SLOT, not per-action,
  so it needs no descriptor field — the panel ranks the visible buttons and each per-act
  digit-press router resolves its key from that rank via `Keybinds::contextual_slot_key`).
  This reverses the Q8 "button-only" ruling; the keyboard slot-bindings coexist
  with mouse clicks, both feeding the one `PendingContextualIntents<A>` dispatch.

### 3. App — the panel button (one file + one line)

- **New module**
  `crates/gdtf_app/src/states/running/game/battlescape/contextual_panel/acts/<act>.rs`:
  the button marker (via `crate::support_item!`), the `ContextualPanelAct` impl
  (`type Marker`, `fn label()`, `const SLOT` — pick an UNUSED `PanelSlot`; slots order
  the column top-to-bottom and must stay unique), and the act's bespoke
  `offer_<act>` system (the scan deciding what to OFFER — advisory only; the sim
  re-gates). Wire the module in `contextual_panel/acts/mod.rs`.
- **One line** in `contextual_panel/plugin.rs`:
  `.add_contextual_act_button::<<Act>Act, _>(acts::<act>::offer_<act>)`. That stamps
  the generic button spawn, visibility toggle, and press router over the descriptor.
- Never add per-act `Without<>` disjointness filters, per-marker visibility bundles, or
  per-marker press queries — the generic systems hold ONE query each, so none are
  needed (the N-squared filter wall must not come back).

### 4. AI arm, or documented why-not (named station — Q6, ruled)

Until lands, the enemy-AI act contract is **move / fire / end-turn, plus
reload-when-landed** — the brain (`crates/gdtf_battle_sim/src/ai/`) writes only `FireRequested` /
`MoveRequested` / `EndTurnRequested` today, and records the reload gap. Every
new act must EITHER add a brain arm that can emit its `*Requested`, OR record here (and
on the ticket) why the AI does not use it yet.

Why-not record for the eight existing contextual acts: they are player-affordance
surfaces pending the AI-acts expansion; none has a brain arm today.

### 5. Tests + test-surface (as the act warrants)

- A headless end-to-end press test in the `crates/gdtf_app/tests/contextual_panel/`
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
`.add_contextual_act_button::<ShoveAct, _>(acts::shove::offer_shove)`, AI why-not
recorded above, and the press/offer/same-frame tests in
`crates/gdtf_app/tests/contextual_panel/shove.rs`.
