//! The [`FieldTicked`] signal + the per-round [`tick_fields`] drain — the area-damage-field
//! clock's message and system (GTW-545, child GTW-41f; GTW-553 moves the per-consequence
//! behaviours into the [`effects::fields`](crate::effects::fields) palette, which this clock
//! invokes generically).

use bevy::prelude::{
    Commands, Component, Entity, Message, MessageWriter, Query, Res, ResMut, With,
};

use super::{FieldDamage, FieldDef, FieldRegistry};
use crate::{
    armor::{ArmorType, Wears, WornBy},
    effects::fields::{ApplyFieldEffect, FieldEffect, OccupantArmor, OccupantDrain},
    ganger::{Hp, LifeState},
    metric::CellLevel,
    occupancy::OccupancyGrid,
    on_death::OnDeathOccurred,
};

/// A ganger **took an area-damage-field tick** this round — a field it stood in drained a flat
/// [`amount`](FieldTicked::amount) of its [`Hp`](crate::ganger::Hp) (GTW-545).
///
/// Emitted by the palette's Drain consequence **once per draining tick** for each non-immune
/// occupant standing on a live field cell (including the lethal tick that empties the HP
/// pool); an empty cell, a dead occupant, or a whole-source-immune occupant drains nothing and
/// emits nothing. The presenter reads this to surface the field damage on screen (the
/// [`DotTicked`](crate::dot::DotTicked) FCT-pop precedent).
///
/// A buffered Bevy **message** (`bevy-traps.md` #4 — NOT the observer `Event`), written with
/// [`MessageWriter`] and read with [`MessageReader`](bevy::prelude::MessageReader), mirroring
/// [`DotTicked`](crate::dot::DotTicked). The [`occupant`](FieldTicked::occupant) is a Bevy
/// [`Entity`] handle (framework plumbing, the only bare type the no-bare-types rule permits in
/// a payload); [`at`](FieldTicked::at) is the domain [`CellLevel`] the field sits at and
/// [`amount`](FieldTicked::amount) the domain [`FieldDamage`] it dealt.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldTicked {
    /// The ganger that took the field tick — the entity whose Hp the field drained.
    pub occupant: Entity,
    /// The `(cell, level)` the field sits at (the occupant stood here; the FCT pop anchor).
    pub at:       CellLevel,
    /// The flat HP the tick drained this round (the field's per-turn damage).
    pub amount:   FieldDamage,
}

impl FieldTicked {
    /// Build a field-tick signal for the `occupant` at `at` that took `amount` HP this round.
    #[must_use]
    pub const fn new(occupant: Entity, at: CellLevel, amount: FieldDamage) -> Self {
        Self {
            occupant,
            at,
            amount,
        }
    }
}

/// Sim bookkeeping: this ganger's field-exposure span is **in progress** — a live field
/// drained it on the most recent [`tick_fields`] round (GTW-572).
///
/// [`tick_fields`] inserts it on the FIRST draining round of a span (alongside the one
/// [`FieldAfflicted`] fact) and removes it the first round the ganger is no longer drained
/// (it stepped off, the field expired, or it died), so stepping back into a hazard later is
/// a NEW span that re-announces. A marker component (no payload — presence IS the fact),
/// never presenter-read: the presenter reads the [`FieldAfflicted`] message.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FieldOngoing;

/// A field **exposure span STARTED** — a live damage field began draining `occupant` at
/// `at` this round (GTW-572).
///
/// The once-at-affliction-start fact the combat log's field line reads (the Q2 ruling:
/// field logs ONCE at affliction start, never per tick). Emitted by [`tick_fields`] exactly
/// once per exposure span — the FIRST round a non-immune occupant standing on a live field
/// is drained — while the per-round [`FieldTicked`] signal keeps firing every draining round
/// for the FCT pop. The sim emits the FACT; the presenter phrases the line (ADR-0001).
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), mirroring
/// [`FieldTicked`]. The [`occupant`](FieldAfflicted::occupant) is a Bevy [`Entity`] handle
/// (framework plumbing, the only bare type the no-bare-types rule permits in a payload).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldAfflicted {
    /// The freshly-exposed ganger — resolved to a display name at the presenter boundary.
    pub occupant: Entity,
    /// The `(cell, level)` of the field that started draining the occupant.
    pub at:       CellLevel,
}

impl FieldAfflicted {
    /// Build a field-exposure-started fact for `occupant` standing on the field at `at`.
    #[must_use]
    pub const fn new(occupant: Entity, at: CellLevel) -> Self {
        Self { occupant, at }
    }
}

/// Drain one round of area-damage from every occupant standing on a live field — the GTW-545
/// per-round clock (`docs/combat/resolution.md` — the area-damage-field beat of GTW-41).
///
/// Run once per full round (ticked at the enemy-phase start, the SAME
/// [`enemy_phase_started`](crate::bleed::enemy_phase_started) cadence as the §9 bleed-out clock
/// and the GTW-544 DOT clock). For each `(cell, level)` carrying a placed field, in order:
///
/// 1. **Find the occupant** — read [`OccupancyGrid::occupant`](crate::occupancy::OccupancyGrid::occupant)
///    at the field cell. An empty cell drains nothing.
/// 2. **Skip a corpse** — a [`LifeState::Dead`] occupant is already a corpse (mutates nothing,
///    emits nothing).
/// 3. **Invoke the GTW-553 consequence palette GENERICALLY** — project the placement's def
///    into its consequence vocabulary
///    ([`FieldEffect::consequences_of`](crate::effects::fields::FieldEffect::consequences_of))
///    and drive the shared [`ApplyFieldEffect`] trait DIRECTLY (synchronous, never a deferred
///    command — the pre-palette drain timing): first the exemption gate (ANY consequence may
///    exempt — the whole-source-immunity skip, over the borrowed [`OccupantArmor`] surface),
///    then the per-turn drain verbs (over the borrowed [`OccupantDrain`] surface — the flat
///    armor-bypassing, RNG-free HP drain, its [`FieldTicked`] signal, and the lethal terminal
///    gate that flips [`LifeState::Dead`] + emits [`OnDeathOccurred`], the GTW-544 DOT-kills
///    precedent). NO per-consequence match lives here — the ONE match over the vocabulary is
///    the palette's own delegation.
///
/// Then, ONCE (after every cell is drained), the [`FieldRegistry`] counts every placement's
/// lifetime down one turn and removes the expired ones (the palette's Duration consequence,
/// via [`PlacedField::tick_down`](super::PlacedField::tick_down)).
///
/// `writer` buffers each [`FieldTicked`]. Pure, render-free, saturating arithmetic — no
/// underflow, no `unwrap`, no pixel. Param-only (`bevy-traps.md` #7): a [`Res<OccupancyGrid>`],
/// a [`ResMut<FieldRegistry>`], the occupant [`Query`], the worn-armor read-only queries, the
/// [`MessageWriter`]s, and [`Commands`] (GTW-572 — the deferred [`FieldOngoing`]
/// span-bookkeeping insert/remove; the marker is only read NEXT round, so the deferred sync
/// point is early enough) — no `&mut World`.
#[expect(
    clippy::too_many_arguments,
    reason = "the field clock reads the occupancy grid + the live registry + the occupant / \
              worn-armor / span-marker queries and writes the tick / death / affliction-start \
              signals plus the deferred span bookkeeping — each a distinct Bevy SystemParam \
              (the dispatch_fire carve-out); bundling would only hide the access set"
)]
pub fn tick_fields(
    grid: Res<OccupancyGrid>,
    mut fields: ResMut<FieldRegistry>,
    mut occupants: Query<(&mut Hp, &mut LifeState, &Wears)>,
    // The QueryData is spelled `&'static` so `&worn` can be lent into the palette's
    // borrowed `OccupantArmor` surface (Query is invariant over its data — the
    // `DeathFanOut` / `VictimRow` precedent); the runtime borrows stay world-scoped.
    worn: Query<&'static ArmorType, With<WornBy>>,
    mut writer: MessageWriter<FieldTicked>,
    // GTW-547: the terminal-death signal — a field tick that KILLS (Hp → 0 → Dead) emits one
    // OnDeathOccurred at the field cell so `resolve_on_death` fans the dead ganger's on-death
    // effect (a field-kill must not silently skip it — the ticket's scope-completeness rule).
    mut deaths: MessageWriter<OnDeathOccurred>,
    // GTW-572: the once-per-span exposure-start fact + the span bookkeeping it keys off (the
    // [`FieldOngoing`] marker: present iff the occupant drained LAST round).
    mut afflicted: MessageWriter<FieldAfflicted>,
    ongoing: Query<Entity, With<FieldOngoing>>,
    mut commands: Commands,
) {
    // Drain each fielded cell's occupant. Snapshot the placements (each cell + a clone of its
    // def) into an owned buffer first so the `&FieldRegistry` borrow is released before the
    // countdown step's `&mut` reborrow below.
    let placements: Vec<(CellLevel, FieldDef)> = fields
        .iter()
        .map(|(cell, placed)| (*cell, placed.def().clone()))
        .collect();

    // GTW-572: the occupants a live field drained THIS round — compared against the
    // FieldOngoing span markers below so a fresh exposure announces exactly once.
    let mut drained_this_round: bevy::platform::collections::HashSet<Entity> =
        bevy::platform::collections::HashSet::default();

    for (cell, def) in placements {
        // (1) The occupant standing on this field cell (None for an empty cell).
        let Some(occupant) = grid.occupant(&cell) else {
            continue;
        };
        // Fetch the occupant's vitals + its worn-armor relationship. A missing row (a stale
        // occupancy slot pointing at a despawned entity) is a no-op (fail-closed).
        let Ok((mut hp, mut life, wears)) = occupants.get_mut(occupant) else {
            continue;
        };
        // (2) A Dead occupant is a corpse — the field does not touch it.
        if *life == LifeState::Dead {
            continue;
        }
        // (3) Project the def into its consequence vocabulary ONCE for this cell, then invoke
        // the palette generically — exemption gate first (ANY consequence may exempt), the
        // per-turn drain verbs after. The behaviours live in `effects::fields`, never here.
        let consequences = FieldEffect::consequences_of(&def);
        let armor = OccupantArmor { wears, worn: &worn };
        if consequences
            .iter()
            .any(|consequence| consequence.exempts_occupant(&armor))
        {
            continue;
        }
        // GTW-572 — span maintenance: the FIRST draining round of an exposure span announces
        // it (one FieldAfflicted) and marks the span; a mid-span round emits only the
        // per-round FieldTicked below (the Q2 ruling: once at affliction start, never per
        // tick). The marker is removed after the loop for anyone no longer drained.
        if !drained_this_round.contains(&occupant) {
            if ongoing.get(occupant).is_err() {
                afflicted.write(FieldAfflicted::new(occupant, cell));
                commands.entity(occupant).insert(FieldOngoing);
            }
            drained_this_round.insert(occupant);
        }

        let mut drain = OccupantDrain {
            hp:     &mut hp,
            life:   &mut life,
            ticks:  &mut writer,
            deaths: &mut deaths,
        };
        for consequence in &consequences {
            consequence.drain_occupant(cell, occupant, &mut drain);
        }
    }

    // GTW-572 — end every span whose occupant was NOT drained this round (it stepped off,
    // the field expired, or it died and was skipped), so a LATER exposure re-announces.
    for entity in &ongoing {
        if !drained_this_round.contains(&entity) {
            commands.entity(entity).remove::<FieldOngoing>();
        }
    }

    // Count every placement's lifetime down one turn and remove the expired ones (the
    // palette's Duration consequence — a Permanent field never counts down). Runs ONCE per
    // round, after the drain — so a field that ticked this round still counts against its
    // lifetime this round.
    fields.tick_down_and_expire();
}
