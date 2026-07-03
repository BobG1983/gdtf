//! The **shared apply contract** for the on-death effect palette — the
//! [`ApplyOnDeathEffect`] trait whose one verb IS an effect's behaviour, and the mutable
//! [`DeathFanOut`] surface the resolver lends it for one death (GTW-552, the on-death
//! sibling of the GTW-558 attachment / GTW-550 injury palettes).
//!
//! Every isolated per-effect `ApplyX` type (one per file in this module) impls this trait;
//! the closed [`OnDeathEffect`](super::OnDeathEffect) serde enum impls it too, by THIN
//! mechanical delegation to each variant's isolated type. The mechanics never match on the
//! enum — the [`resolve_on_death`](crate::on_death::resolve_on_death) fixpoint loop invokes
//! this trait generically, DIRECTLY on its same-frame cascade drain (synchronous — never a
//! deferred command — so a lethal fan's fresh deaths land on the SAME work-queue the loop
//! is draining, the GTW-547 cascade cadence unchanged).

use bevy::prelude::Query;

use crate::{
    fields::{FieldDefRegistry, FieldRegistry},
    ganger::{Hp, LifeState},
    metric::CellLevel,
    occupancy::OccupancyGrid,
    on_death::OnDeathOccurred,
};

/// The victim-surface query row an on-death fan mutates — the struck ganger's
/// `(`[`Hp`]`, `[`LifeState`]`)` bundled into one `QueryData` tuple alias so the `Query`
/// stays under clippy's type-complexity gate. Mutable on both (a blast drains [`Hp`] and
/// flips [`LifeState`] to [`LifeState::Dead`] on a lethal fan).
pub type VictimRow = (&'static mut Hp, &'static mut LifeState);

/// The mutable **fan-out surface** an on-death effect fans into — the battle surfaces
/// [`resolve_on_death`](crate::on_death::resolve_on_death) lends out for the duration of
/// ONE death's fan (GTW-552).
///
/// A borrowed VIEW over the resolver's own `SystemParam`s, never effect-owned state: the
/// occupancy / victim pair is the blast surface (who stands where, whose [`Hp`] drains),
/// the field pair is the hazard surface (the GTW-545 catalog + placement registry), and
/// [`cascade`](Self::cascade) is the resolver's SAME-FRAME work-queue — an effect that
/// KILLS pushes the fresh [`OnDeathOccurred`] here (a typed work item the caller's fixpoint
/// loop drains, visited-set-guarded), never back through the message buffer (whose reader
/// cursor would not re-observe it this frame — the GTW-547 cascade cadence). An effect that
/// needs a genuinely NEW surface adds a field here (one new view field), keeping the
/// resolver the one owner of the world access.
pub struct DeathFanOut<'a, 'w, 's> {
    /// Who stands where — the cell → occupant read a blast resolves its victims through.
    pub grid:       &'a OccupancyGrid,
    /// The struck gangers' mutable `(Hp, LifeState)` surface a blast drains.
    pub victims:    &'a mut Query<'w, 's, VictimRow>,
    /// The GTW-545 live-field placement registry a field-leaving effect spawns into.
    pub fields:     &'a mut FieldRegistry,
    /// The GTW-545 field CATALOG (app/Load-owned, so possibly absent — a battle with no
    /// field content has none; an effect that needs it fans nothing when it is `None`,
    /// fail-closed).
    pub field_defs: Option<&'a FieldDefRegistry>,
    /// The resolver's same-frame cascade work-queue — a lethal fan pushes each fresh death
    /// here as a typed [`OnDeathOccurred`] work item (see [`DeathFanOut`]'s type docs).
    pub cascade:    &'a mut Vec<OnDeathOccurred>,
}

/// One on-death effect's **isolated behaviour** — the trait each concrete effect type
/// impls, its one verb BEING the effect (GTW-552; the effect-isolation contract, the
/// GTW-558 attachment-palette precedent).
///
/// The whole point: an effect's logic lives in exactly ONE place — its own type's
/// [`fan_at`](Self::fan_at) in its own palette file — not in a central `match`, a
/// folder-fn, or a scatter of authoring steps. The [`OnDeathEffect`](super::OnDeathEffect)
/// serde enum impls this by DELEGATING each variant to its isolated type (the palette's ONE
/// mechanical match), and [`resolve_on_death`](crate::on_death::resolve_on_death) invokes
/// it generically over the borrowed [`DeathFanOut`] surface.
pub trait ApplyOnDeathEffect {
    /// **Fan** this effect at the death `(cell, level)` — its whole behaviour, applied
    /// synchronously through the borrowed [`DeathFanOut`] surface (never a deferred
    /// command: the resolver's same-frame cascade fixpoint depends on the fan's kills
    /// landing on [`cascade`](DeathFanOut::cascade) before the loop's next pop).
    ///
    /// Deterministic and render-free: an implementation takes NO RNG (the GTW-547
    /// byte-stable guarantee — a demo fan must not perturb the seeded autobattle stream)
    /// and fans nothing when its target surface is unavailable (fail-closed, never a
    /// panic).
    fn fan_at(&self, at: CellLevel, fan_out: &mut DeathFanOut<'_, '_, '_>);
}
