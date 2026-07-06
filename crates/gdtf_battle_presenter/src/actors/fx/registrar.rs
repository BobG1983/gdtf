//! The compile-time per-message FX-reader registrar (GTW-623 C3) — the machinery that
//! replaced the six hand-cloned flash-family gate stanzas in the renderer plugin.
//!
//! Each transient FX reader ([`read_bleeding`](super::read_bleeding) /
//! [`read_armor_broken`](super::read_armor_broken) /
//! [`read_cover_destroyed`](super::read_cover_destroyed) /
//! [`read_melee_resolved`](super::read_melee_resolved) /
//! [`read_fall_occurred`](super::read_fall_occurred) /
//! [`read_throw_resolved`](super::read_throw_resolved)) drains ONE sim-owned message
//! buffer and spawns short-lived FX sprites from the render tables. Registration is
//! COMPILE-TIME generic (the [`ConsequenceFctAppExt`](super::ConsequenceFctAppExt)
//! exemplar — no runtime descriptor table): `app.add_fx_reader::<M, _>(system)` is the
//! ONE line a reader needs, and it carries the whole gate shape — the shared render
//! gate plus a REAL `Messages<M>` gate. The registrar NEVER calls `add_message`
//! (the GTW-572 C4 convention): the sim's plugins register every sim-owned buffer in a
//! live battle, and a presenter-only headless harness that omits the buffer keeps that
//! reader INERT instead of failing param validation (`bevy-traps.md` #1 / #4).

use bevy::{
    ecs::{
        message::{Message, Messages},
        schedule::SystemCondition,
        system::ScheduleSystem,
    },
    prelude::{App, IntoScheduleConfigs, Update, resource_exists},
};
use gdtf_battle_sim::BattleInProgress;

use super::roles::EffectRoles;
use crate::{PresenterSystems, TopDownAtlases};

/// The per-reader registrar (GTW-623 C3): `app.add_fx_reader::<M, _>(system)` is the ONE
/// registration line a transient FX reader needs — mirroring
/// [`ConsequenceFctAppExt`](super::ConsequenceFctAppExt).
pub trait FxReaderAppExt {
    /// Register `reader` — a system draining `Messages<M>` — into the
    /// [`PresenterSystems::Overlay`] stage (its ordering comes from STAGE MEMBERSHIP,
    /// never a pairwise edge — GTW-623 C1/C2) with today's gate shape:
    /// `BattleInProgress` (FX belong to a live battle) AND the [`EffectRoles`] data
    /// table AND [`TopDownAtlases`] (the render resources every flash spawn reads;
    /// absent under `MinimalPlugins`, so a no-asset app simply does not draw —
    /// `bevy-traps.md` #1) AND the REAL `Messages<M>` buffer gate (a `MessageReader<M>`
    /// panics param validation without its buffer — `bevy-traps.md` #4).
    ///
    /// NEVER calls `add_message`: the sim's plugins register every sim-owned buffer in
    /// a live battle, and a presenter-only headless harness that omits the buffer keeps
    /// this reader inert (the GTW-572 C4 convention, pinned by the `fx_draw`
    /// registrar-contract tests). A reader needing an EXTRA gate composes it onto the
    /// system it passes in (run conditions AND together): e.g.
    /// `add_fx_reader::<FallOccurred, _>(read_fall_occurred.run_if(resource_exists::<FxTuning>))`.
    fn add_fx_reader<M: Message, Marker>(
        &mut self,
        reader: impl IntoScheduleConfigs<ScheduleSystem, Marker>,
    ) -> &mut Self;
}

impl FxReaderAppExt for App {
    fn add_fx_reader<M: Message, Marker>(
        &mut self,
        reader: impl IntoScheduleConfigs<ScheduleSystem, Marker>,
    ) -> &mut Self {
        self.add_systems(
            Update,
            reader.in_set(PresenterSystems::Overlay).run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<EffectRoles>)
                    .and_then(resource_exists::<TopDownAtlases>)
                    .and_then(resource_exists::<Messages<M>>),
            ),
        );
        self
    }
}
