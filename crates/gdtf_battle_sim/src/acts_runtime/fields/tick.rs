//! The [`FieldTicked`] signal + the per-round [`tick_fields`] drain — the area-damage-field
//! clock's message and system (GTW-545, child GTW-41f).

use bevy::prelude::{Entity, Message, MessageWriter, Query, Res, ResMut, With};

use super::{FieldDamage, FieldRegistry, ImmuneArmorTypes};
use crate::{
    armor::{ArmorType, Wears, WornBy},
    ganger::{Hp, LifeState},
    metric::CellLevel,
    occupancy::OccupancyGrid,
    on_death::OnDeathOccurred,
};

/// A ganger **took an area-damage-field tick** this round — a field it stood in drained a flat
/// [`amount`](FieldTicked::amount) of its [`Hp`](crate::ganger::Hp) (GTW-545).
///
/// Emitted by [`tick_fields`] **once per draining tick** for each non-immune occupant standing
/// on a live field cell (including the lethal tick that empties the HP pool); an empty cell, a
/// dead occupant, or a whole-source-immune occupant drains nothing and emits nothing. The
/// presenter reads this to surface the field damage on screen (the
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
/// 3. **Whole-source immunity (the GTW-545 NEW mechanism)** — if ANY of the occupant's worn
///    armor pieces carries an [`ArmorType`] in the field def's
///    [`immune_armor_types`](crate::acts_runtime::fields::FieldDef::immune_armor_types), the
///    occupant takes ZERO damage (the sealed suit protects you). No per-hit matchup, no injury
///    roll, no RNG.
/// 4. **Drain Hp DIRECTLY** — otherwise subtract the field's per-turn damage from the
///    occupant's [`Hp`](crate::ganger::Hp) (`saturating_sub`, floors at `0` — no underflow),
///    with NO armor matchup, NO injury roll, and NO RNG (the deterministic field tick), and
///    emit one [`FieldTicked`] carrying the occupant + the `(cell, level)` + the amount.
/// 5. **Terminal gate — a field drain that empties HP KILLS** — if the drain emptied the
///    occupant's [`Hp`](crate::ganger::Hp) to `0`, flip it to [`LifeState::Dead`] (the
///    GTW-544 DOT-kills precedent: a persistent-zone drain that brings HP to `0` is lethal,
///    NOT a down).
///
/// Then, ONCE (after every cell is drained), the [`FieldRegistry`] counts every `Turns` field
/// down one turn and removes the ones that expired; a `Permanent` field never counts down.
///
/// `writer` buffers each [`FieldTicked`]. Pure, render-free, saturating arithmetic — no
/// underflow, no `unwrap`, no pixel. Param-only (`bevy-traps.md` #7): a [`Res<OccupancyGrid>`],
/// a [`ResMut<FieldRegistry>`], the occupant [`Query`], the worn-armor read-only queries, and a
/// [`MessageWriter`] — no `&mut World`, no [`Commands`](bevy::prelude::Commands).
pub fn tick_fields(
    grid: Res<OccupancyGrid>,
    mut fields: ResMut<FieldRegistry>,
    mut occupants: Query<(&mut Hp, &mut LifeState, &Wears)>,
    worn: Query<&ArmorType, With<WornBy>>,
    mut writer: MessageWriter<FieldTicked>,
    // GTW-547: the terminal-death signal — a field tick that KILLS (Hp → 0 → Dead) emits one
    // OnDeathOccurred at the field cell so `resolve_on_death` fans the dead ganger's on-death
    // effect (a field-kill must not silently skip it — the ticket's scope-completeness rule).
    mut deaths: MessageWriter<OnDeathOccurred>,
) {
    // Drain each fielded cell's occupant. Read the placements into an owned buffer first (the
    // cell, its per-turn damage, and its immune set) so the `&FieldRegistry` borrow is released
    // before the countdown step's `&mut` reborrow below.
    let placements: Vec<(CellLevel, FieldDamage, ImmuneArmorTypes)> = fields
        .iter()
        .map(|(cell, placed)| {
            (
                *cell,
                placed.def().damage,
                placed.def().immune_armor_types.clone(),
            )
        })
        .collect();

    for (cell, amount, immune) in placements {
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
        // (3) Whole-source immunity — the GTW-545 NEW mechanism. If ANY worn piece's
        // ArmorType is in the field's immune set, the occupant takes zero damage. Look up each
        // piece's ArmorType through the worn-piece query (the melee resolve precedent).
        let is_immune = wears.pieces().any(|piece| {
            worn.get(piece)
                .is_ok_and(|armor_type| immune.contains(armor_type))
        });
        if is_immune {
            continue;
        }
        // (4) Drain Hp DIRECTLY — saturating at 0 (Hp is unsigned; a lethal tick floors it,
        // never underflows). NO armor matchup, NO injury roll, NO RNG (the deterministic field
        // tick). Emit one FieldTicked carrying the occupant, the cell, and the amount.
        *hp = Hp::new(hp.saturating_sub(*amount));
        writer.write(FieldTicked::new(occupant, cell, amount));
        // (5) Terminal gate — a field drain that empties Hp KILLS (flip to Dead, the GTW-544
        // DOT-kills precedent for a persistent-zone drain).
        if *hp == Hp::new(0) {
            *life = LifeState::Dead;
            // GTW-547: emit the terminal-death signal at the field cell (the occupant stood
            // here) so the dead ganger's authored on-death effect fans.
            deaths.write(OnDeathOccurred::new(occupant, cell));
        }
    }

    // Count every Turns field down one turn and remove the expired ones (a Permanent field
    // never counts down). Runs ONCE per round, after the drain — so a field that ticked this
    // round still counts against its lifetime this round.
    fields.tick_down_and_expire();
}
