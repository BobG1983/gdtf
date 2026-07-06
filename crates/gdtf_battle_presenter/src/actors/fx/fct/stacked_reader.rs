//! The ONE generic stacked-pop reader (GTW-572 C2) + the compile-time per-family registrar
//! (C4) — the machinery that replaced the six hand-rolled consequence reader clones.
//!
//! [`read_consequence_fct::<C>`] is the whole reader: drain the family's signal, classify it
//! through the family's [`ConsequenceFct`] impl, resolve the anchor (FAIL-CLOSED on a
//! [`PopAnchor::GangerPosition`] whose entity has no live
//! [`Position`](gdtf_battle_sim::Position) — the pop is DROPPED, never spawned at a default
//! position), claim the next slot from the SHARED per-frame
//! [`FctStackCounter`](super::stack::FctStackCounter) (C3 — cross-family same-cell pops fan
//! out instead of overlapping), and spawn the pop with the hot-reloadable
//! [`FxTuning`](super::super::FxTuning) lifetime + rise.
//!
//! Registration is COMPILE-TIME generic (P4 — no runtime descriptor table):
//! [`register_consequence_fct_core`] wires the shared pieces ONCE (the counter resource, the
//! reset system explicitly `.before` the reader set — `bevy-traps.md` #3, the set's
//! placement in the [`PresenterSystems::Overlay`] stage), and each family is one
//! [`ConsequenceFctAppExt::add_consequence_fct`] line. The registrar NEVER calls
//! `add_message` — a presenter-only headless harness that omits a family's `Messages<M>`
//! buffer keeps that reader INERT (the `run_if` gate), which the inertness tests rely on; in
//! a live battle the sim's plugins register every buffer.

use bevy::{
    ecs::{message::Messages, schedule::SystemCondition},
    prelude::{
        App, Commands, IntoScheduleConfigs, MessageReader, Query, Res, ResMut, SystemSet, Update,
        resource_exists,
    },
};
use gdtf_battle_sim::{BattleInProgress, Position};

use super::{
    super::FxTuning,
    pop::{ConsequenceFct, PopAnchor},
    stack::{FctStackCounter, reset_fct_stacks},
    text::spawn_floating_text,
};
use crate::PresenterSystems;

/// The consequence-FCT scheduling sets (GTW-572 C3): the shared counter [`Reset`](Self::Reset)
/// runs strictly before the whole per-family [`Read`](Self::Read) set — the EXPLICIT ordering
/// (`bevy-traps.md` #3) that makes the per-frame stack slots well-defined.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsequenceFctSystems {
    /// Clears the shared [`FctStackCounter`] — the frame boundary.
    Reset,
    /// Every generic per-family reader ([`read_consequence_fct::<C>`]).
    Read,
}

/// `Update` (`ConsequenceFctSystems::Read`, in the `PresenterSystems::Overlay` stage): the ONE generic
/// stacked-pop reader — drain family `C`'s signal and spawn one rise-and-fade pop per
/// message (GTW-572 C2).
///
/// For each drained signal it classifies via [`C::classify`](ConsequenceFct::classify) (the
/// family's pure mapping, unit-tested in the family file), resolves the anchor — a
/// [`PopAnchor::Carried`] cell directly, a [`PopAnchor::GangerPosition`] through the
/// read-only `Query<&Position>` via the canonical
/// [`CellLevel::split`](gdtf_battle_sim::CellLevel::split) (GTW-565), FAIL-CLOSED: no
/// `Position` → no pop, no panic — claims the next slot from the SHARED per-frame
/// [`FctStackCounter`] (C3), and spawns via [`spawn_floating_text`] with the family's
/// classified emphasis and the hot-reloadable [`FxTuning`] lifetime + rise (GTW-327).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], the [`MessageReader`], the read-only
/// anchor query, the shared counter, and [`Res<FxTuning>`]. Its registrar gate
/// ([`ConsequenceFctAppExt::add_consequence_fct`]) adds `BattleInProgress` + the family's
/// `Messages<C::Signal>` buffer + `FxTuning`, so the params are always valid
/// (`bevy-traps.md` #1 / #4).
pub fn read_consequence_fct<C: ConsequenceFct>(
    mut commands: Commands,
    mut signals: MessageReader<C::Signal>,
    positions: Query<&Position>,
    mut stacks: ResMut<FctStackCounter>,
    tuning: Res<FxTuning>,
) {
    for signal in signals.read() {
        let pop = C::classify(signal);
        // Resolve the anchor. GangerPosition is FAIL-CLOSED: an unresolvable entity DROPS
        // the pop — never a pop at a default position (GTW-572 C1).
        let at = match pop.anchor() {
            PopAnchor::Carried(at) => at,
            PopAnchor::GangerPosition(entity) => {
                let Ok(position) = positions.get(entity) else {
                    continue;
                };
                // Position derefs to its CellLevel key (GTW-565).
                **position
            }
        };
        // The SHARED per-frame counter (C3): cross-family pops on one cell fan out.
        let slot = stacks.next(at);
        let (cell, level) = at.split();
        spawn_floating_text(
            &mut commands,
            pop.text().clone(),
            pop.color(),
            pop.emphasis(),
            cell,
            level,
            slot,
            tuning.fct_ttl_seconds,
            tuning.fct_rise_rate,
        );
    }
}

/// Wire the SHARED consequence-FCT core ONCE (GTW-572 C3): the [`FctStackCounter`] resource,
/// the [`ConsequenceFctSystems`] set chain (`Reset` strictly `.before(Read)`, both inside
/// the [`PresenterSystems::Overlay`] stage (GTW-623 — the FCT palette draws over the
/// composed scene) — the EXPLICIT ordering of `bevy-traps.md` #3), and the
/// [`reset_fct_stacks`] system. Called once by `TopDownRendererPlugin::build` before the
/// per-family [`ConsequenceFctAppExt::add_consequence_fct`] lines.
pub fn register_consequence_fct_core(app: &mut App) {
    app.init_resource::<FctStackCounter>()
        .configure_sets(
            Update,
            (
                ConsequenceFctSystems::Reset.in_set(PresenterSystems::Overlay),
                ConsequenceFctSystems::Read.in_set(PresenterSystems::Overlay),
            )
                .chain(),
        )
        .add_systems(
            Update,
            reset_fct_stacks.in_set(ConsequenceFctSystems::Reset),
        );
}

/// The per-family registrar (GTW-572 C4): `app.add_consequence_fct::<Family>()` is the ONE
/// registration line a consequence family needs.
pub trait ConsequenceFctAppExt {
    /// Register family `C`'s generic stacked-pop reader with today's gate shape —
    /// `BattleInProgress` (pops belong to a live battle) + the family's `Messages<M>` buffer
    /// (a [`MessageReader`] panics param validation without it — `bevy-traps.md` #1 / #4) +
    /// the hot-reloadable [`FxTuning`] the pop lifetime/rise read.
    ///
    /// NEVER calls `add_message`: a presenter-only headless harness that omits the buffer
    /// keeps this reader inert (the sim's plugins register every buffer in a live battle).
    fn add_consequence_fct<C: ConsequenceFct>(&mut self) -> &mut Self;
}

impl ConsequenceFctAppExt for App {
    fn add_consequence_fct<C: ConsequenceFct>(&mut self) -> &mut Self {
        self.add_systems(
            Update,
            read_consequence_fct::<C>
                .in_set(ConsequenceFctSystems::Read)
                .run_if(
                    resource_exists::<BattleInProgress>
                        .and_then(resource_exists::<Messages<C::Signal>>)
                        .and_then(resource_exists::<FxTuning>),
                ),
        );
        self
    }
}
