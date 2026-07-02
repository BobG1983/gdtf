//! The [`DotTicked`] signal + the per-round [`tick_dot`] drain — the damage-over-time
//! clock's message and system (GTW-544, child GTW-41e).

use bevy::prelude::{Entity, Message, MessageWriter, Query};

use crate::{
    ganger::{Hp, LifeState, Position},
    metric::CellLevel,
    weapon::{Dot, DotDamage, DotTurns},
};

/// The [`tick_dot`] per-ganger query row — the afflicted ganger's surfaces bundled into one
/// `QueryData` tuple alias so the `Query` stays under clippy's type-complexity gate: the
/// [`Entity`], the mutable [`Hp`] (the DOT drains it DIRECTLY), the mutable [`LifeState`] (the
/// terminal kill gate), the mutable [`Dot`] (the affliction — its turns decrement, it is
/// removed at zero), and the read-only [`Position`] (the `(cell, level)` the [`DotTicked`]
/// signal carries for the presenter's floating-combat-text pop).
type DotRow = (
    Entity,
    &'static mut Hp,
    &'static mut LifeState,
    &'static mut Dot,
    &'static Position,
);

/// A ganger **took a DOT tick** this round — a damage-over-time affliction drained a flat
/// [`amount`](DotTicked::amount) of its [`Hp`](crate::ganger::Hp) (GTW-544).
///
/// Emitted by [`tick_dot`] **once per draining tick** for each afflicted ganger (including
/// the lethal tick that empties the HP pool); a ganger with no [`Dot`](crate::weapon::Dot),
/// or a dead one, drains nothing and emits nothing. The presenter reads this to surface the
/// DOT damage on screen (the [`Bleeding`](crate::bleed::Bleeding) FCT-pop precedent).
///
/// A buffered Bevy **message** (`bevy-traps.md` #4 — NOT the observer `Event`), written with
/// [`MessageWriter`] and read with [`MessageReader`](bevy::prelude::MessageReader), mirroring
/// [`Bleeding`](crate::bleed::Bleeding). The [`ganger`](DotTicked::ganger) is a Bevy
/// [`Entity`] handle (framework plumbing, the only bare type the no-bare-types rule permits
/// in a payload); [`at`](DotTicked::at) is the domain [`CellLevel`] the tick landed at and
/// [`amount`](DotTicked::amount) the domain [`DotDamage`] it dealt.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DotTicked {
    /// The ganger that took the DOT tick — the entity whose Hp the tick drained.
    pub ganger: Entity,
    /// The `(cell, level)` the ganger occupied when the tick landed (the FCT pop anchor).
    pub at:     CellLevel,
    /// The flat HP the tick drained this round (the DOT's per-turn damage).
    pub amount: DotDamage,
}

impl DotTicked {
    /// Build a DOT-tick signal for the `ganger` at `at` that took `amount` HP this round.
    #[must_use]
    pub const fn new(ganger: Entity, at: CellLevel, amount: DotDamage) -> Self {
        Self { ganger, at, amount }
    }
}

/// Drain one round of damage-over-time from every afflicted ganger — the GTW-544 per-round
/// clock (`docs/combat/resolution.md` — the DOT beat of GTW-41).
///
/// Run once per full round (ticked at the enemy-phase start, the SAME
/// [`enemy_phase_started`](crate::bleed::enemy_phase_started) cadence as the §9 bleed-out
/// clock). For each ganger carrying a [`Dot`](crate::weapon::Dot), in order:
///
/// 1. **Skip a corpse** — a [`LifeState::Dead`] ganger is already a corpse (the once-only
///    property: once Dead, the next tick skips it). Mutates nothing, emits nothing.
/// 2. **Drain Hp DIRECTLY** — subtract the DOT's per-turn damage from the ganger's
///    [`Hp`](crate::ganger::Hp) (`saturating_sub`, floors at `0` — no underflow), with NO
///    armor matchup, NO injury roll, and NO RNG (the deterministic DOT tick), and emit one
///    [`DotTicked`] carrying the ganger + its `(cell, level)` + the amount.
/// 3. **Decrement the clock** — decrement the DOT's remaining turns.
/// 4. **Terminal gate — the GTW-544 locked design: DOT KILLS** — if the drain emptied the
///    ganger's [`Hp`](crate::ganger::Hp) to `0`, flip it to [`LifeState::Dead`] (NOT `Downed`
///    — the ticket's locked spec: "if `Hp` hits 0 flip `LifeState` to Dead"). Unlike the weapon
///    HP-loss path (`Hp` → 0 downs) and the injury bleed (`Hp` → 0 downs), a DOT's HP depletion
///    is lethal.
/// 5. **Remove the exhausted affliction** — when the remaining turns reach `0`, remove the
///    [`Dot`](crate::weapon::Dot) component (the affliction has run its profile turn count).
///
/// `writer` buffers each [`DotTicked`]. Pure, render-free, saturating arithmetic — no
/// underflow, no `unwrap`, no pixel. Param-only (`bevy-traps.md` #7): a [`Query`], a
/// [`MessageWriter`], and — for the component removal — [`Commands`](bevy::prelude::Commands).
pub fn tick_dot(
    mut q: Query<DotRow>,
    mut writer: MessageWriter<DotTicked>,
    mut commands: bevy::prelude::Commands,
) {
    for (entity, mut hp, mut life, mut dot, position) in &mut q {
        // (1) A Dead ganger is a corpse — the DOT does not touch it (the once-only property:
        // once Dead, the next tick skips it). A defensively exhausted DOT (zero turns) is
        // likewise inert until removed below.
        if *life == LifeState::Dead || !dot.is_active() {
            continue;
        }

        // (2) Drain Hp DIRECTLY — saturating at 0 (Hp is unsigned; a lethal tick floors it,
        // never underflows). NO armor matchup, NO injury roll, NO RNG (the deterministic DOT
        // tick). Emit one DotTicked carrying the ganger, its (cell, level), and the amount.
        let amount = dot.per_turn_damage;
        *hp = Hp::new(hp.saturating_sub(*amount));
        // `position` is `&Position`; `**position` derefs `Position → CellLevel` (the DotTicked
        // anchor). One fewer deref than `***position` (which would reach the inner IVec3).
        writer.write(DotTicked::new(entity, **position, amount));

        // (3) Decrement the clock by one turn (saturating — never below 0).
        dot.remaining_turns = DotTurns::new(dot.remaining_turns.saturating_sub(1));

        // (4) Terminal gate — the GTW-544 locked design: a DOT tick that empties Hp KILLS
        // (flip to Dead, NOT Downed). This diverges deliberately from the weapon / injury-bleed
        // Hp→0-downs gate: the ticket's locked spec says the DOT's HP depletion is lethal.
        if *hp == Hp::new(0) {
            *life = LifeState::Dead;
        }

        // (5) Remove the exhausted affliction — when the profile turn count runs out, drop the
        // Dot component (via Commands; the deferred removal settles at the frame's sync point).
        // A dead ganger's DOT is likewise removed (it will never tick again).
        if !dot.is_active() || *life == LifeState::Dead {
            commands.entity(entity).remove::<Dot>();
        }
    }
}
