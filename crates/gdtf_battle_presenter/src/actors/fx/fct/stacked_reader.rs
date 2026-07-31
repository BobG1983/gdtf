//! The ONE generic stacked-pop reader (GTW-572 C2) + the compile-time per-family registrar
//! (C4) — the machinery that replaced the six hand-rolled consequence reader clones.
//!
//! [`read_consequence_fct::<C>`] is the whole reader: drain the family's PLAYED signal
//! ([`Played<C::Signal>`](Played) — the fact at the moment the playback cursor shows it,
//! GTW-889), classify it
//! through the family's [`ConsequenceFct`] impl, resolve the anchor over the DRAWN cell
//! ([`DrawnPosition`] — where the cursor has shown the ganger, GTW-889; FAIL-CLOSED on a
//! [`PopAnchor::GangerPosition`] whose entity has no
//! [`Position`](gdtf_battle_sim::ganger::Position) at all — the pop is DROPPED, never spawned at a
//! default position), claim the next slot from the lifetime-aware
//! [`FctSlotAllocator`](super::slot_allocator::FctSlotAllocator) (GTW-793 — the slot ABOVE
//! every pop still ALIVE on the cell, so same-cell pops fan out across FRAMES, not just within
//! one frame like the retired per-frame counter), and spawn the pop with the hot-reloadable
//! [`FxTuning`](super::super::FxTuning) lifetime + rise.
//!
//! Registration is COMPILE-TIME generic (P4 — no runtime descriptor table):
//! [`register_consequence_fct_core`] wires the shared pieces ONCE (the reader set's placement
//! in the [`PresenterSystems::Overlay`] stage, ordered `.after(animate_floating_text)` so the
//! allocator counts pops after this frame's despawns flush — `bevy-traps.md` #3), and each
//! family is one [`ConsequenceFctAppExt::add_consequence_fct`] line, plus the act-log deed
//! and `Played<Signal>` registration that reader now needs (see the palette recipe in
//! [`families`](super::families)). The registrar NEVER calls `add_message` for a SIM buffer
//! — a presenter-only headless harness that omits a family's `Messages<M>` buffer keeps that
//! reader INERT (the `run_if` gate), which the inertness tests rely on; in a live battle the
//! sim's plugins register every buffer.

use bevy::{
    ecs::{message::Messages, schedule::SystemCondition},
    prelude::{
        App, Commands, IntoScheduleConfigs, MessageReader, Query, Res, SystemSet, Update,
        resource_exists,
    },
};
use gdtf_battle_sim::prelude::{BattleInProgress, Position};

use super::{
    super::FxTuning,
    pop::{ConsequenceFct, PopAnchor},
    slot_allocator::FctSlotAllocator,
    text::{animate_floating_text, spawn_floating_text},
};
use crate::{
    PresenterSystems,
    playback::{DrawnPosition, Played},
};

/// The anchor-resolution query columns: the ganger's live sim [`Position`] and the
/// [`DrawnPosition`] mirror the playback cursor writes.
///
/// The mirror is the one the pop uses; the live position is read ONLY as the pre-seed
/// fallback (see [`read_consequence_fct`]). Factored into a [`QueryData`] tuple so the
/// reader's signature stays under clippy's `type_complexity` gate.
///
/// [`QueryData`]: bevy::ecs::query::QueryData
type AnchorData = (&'static Position, Option<&'static DrawnPosition>);

/// The consequence-FCT scheduling set. The [`Read`](Self::Read) set (every generic per-family
/// reader) is ordered `.after(animate_floating_text)` (GTW-793) so its [`FctSlotAllocator`]
/// counts pops after this frame's despawns have flushed — the EXPLICIT ordering `bevy-traps.md`
/// #3 requires.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsequenceFctSystems {
    /// Every generic per-family reader ([`read_consequence_fct::<C>`]).
    Read,
}

/// `Update` (`ConsequenceFctSystems::Read`, in the `PresenterSystems::Overlay` stage): the ONE generic
/// stacked-pop reader — drain family `C`'s PLAYED signal and spawn one rise-and-fade pop per
/// message (GTW-572 C2, paced in GTW-889).
///
/// # It drains `Played<C::Signal>`, not the raw sim message (GTW-889)
///
/// The signal a family classifies is a sim fact, and the sim resolves a whole exchange in
/// one tick. Draining the raw buffer put every pop on screen at SIM time: a `"SUPPRESSED"`
/// tag and an injury name appeared at the top of the enemy turn, ahead of the shots that
/// caused them, because those shots were still being played out by the playback cursor.
/// Reading [`Played<C::Signal>`](Played) — the same wrapper the combat-log forwarders and
/// every other FX reader already take — puts each pop at the moment the cursor SHOWS its
/// fact, so a pop can no longer precede its cause.
///
/// # The anchor is the DRAWN cell, not the sim's live one (GTW-889)
///
/// Moving the pop onto the cursor's clock moves WHEN it appears; the anchor decides WHERE,
/// and it has to move with it. The sim runs ahead of the cursor — that is this ticket's
/// whole premise — so by the time the cursor plays an injury the sim may already have
/// walked that ganger several cells on. Reading the live
/// [`Position`](gdtf_battle_sim::ganger::Position) would drop a `"LACERATED"` tag on a cell
/// the sprite has not been drawn at yet, because the sprite is moved from the mirror
/// ([`move_ganger_sprites`](crate::move_ganger_sprites) filters
/// `Changed<DrawnPosition>`). So the anchor reads [`DrawnPosition`] — the cell the cursor
/// has SHOWN — and the pop lands on the body the player can see. It is the same swap
/// [`resolve_ganger_visibility`](crate::resolve_ganger_visibility) makes for life state,
/// and the same property the SHOT pops already get by capturing their anchor at the played
/// shot and threading it through the staggered impact (`fct::reader::anchor::anchor_cell`).
///
/// The live `Position` is still queried, for two jobs: it is the fail-closed existence
/// check (an entity with no `Position` is not a drawable ganger — no pop, no panic), and it
/// is the fallback for the one frame before
/// [`seed_drawn_state`](crate::playback::seed_drawn_state) has given a freshly spawned
/// ganger its mirror, where the two values are equal by construction anyway.
///
/// For each drained signal it classifies via [`C::classify`](ConsequenceFct::classify) (the
/// family's pure mapping, unit-tested in the family file), resolves the anchor — a
/// [`PopAnchor::Carried`] cell directly, a [`PopAnchor::GangerPosition`] through the
/// read-only anchor query via the canonical
/// [`CellLevel::split`](gdtf_battle_sim::metric::CellLevel::split) (GTW-565), FAIL-CLOSED: no
/// `Position` → no pop, no panic — claims the next slot from the lifetime-aware
/// [`FctSlotAllocator`] (GTW-793 — the slot ABOVE the pops still ALIVE on the cell, spanning
/// FRAMES, not the retired per-frame counter), and spawns via [`spawn_floating_text`] with the
/// family's classified emphasis and the hot-reloadable [`FxTuning`] lifetime + rise (GTW-327).
///
/// ORDERING (`bevy-traps.md` #3): the reader's [`ConsequenceFctSystems::Read`] set runs
/// `.after(animate_floating_text)` (wired in [`register_consequence_fct_core`]) so a pop
/// expiring this frame is despawned — and its command flushed — BEFORE the allocator counts,
/// the ordering convention [`FctSlotAllocator`] requires (GTW-792).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], the [`MessageReader`], the read-only
/// anchor query, the [`FctSlotAllocator`], and [`Res<FxTuning>`]. Its registrar gate
/// ([`ConsequenceFctAppExt::add_consequence_fct`]) adds `BattleInProgress` + the family's
/// `Messages<C::Signal>` buffer + `FxTuning`, so the params are always valid
/// (`bevy-traps.md` #1 / #4).
pub fn read_consequence_fct<C: ConsequenceFct>(
    mut commands: Commands,
    mut signals: MessageReader<Played<C::Signal>>,
    anchors: Query<AnchorData>,
    allocator: FctSlotAllocator,
    tuning: Res<FxTuning>,
) {
    for signal in signals.read() {
        // `Played<S>` derefs to the wrapped sim fact, so the family's classify is reached
        // unchanged — only the buffer the fact arrives on moved.
        let pop = C::classify(&**signal);
        // Resolve the anchor. GangerPosition is FAIL-CLOSED: an unresolvable entity DROPS
        // the pop — never a pop at a default position (GTW-572 C1).
        let at = match pop.anchor() {
            PopAnchor::Carried(at) => at,
            PopAnchor::GangerPosition(entity) => {
                let Ok((position, drawn)) = anchors.get(entity) else {
                    continue;
                };
                // The SHOWN cell (GTW-889) — the sim may already have walked this ganger on.
                // Position derefs to its CellLevel key (GTW-565); DrawnPosition::position
                // hands back the mirrored one. The live value is used only while the cursor
                // has seeded no mirror for this ganger yet, where the two agree anyway.
                drawn.map_or(**position, |drawn| *drawn.position())
            }
        };
        // The lifetime-aware allocator (GTW-793): the next slot ABOVE every pop still ALIVE on
        // the cell — so same-cell pops fan out across FRAMES, not just within one frame.
        let slot = allocator.next_slot(at);
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

/// Wire the SHARED consequence-FCT core ONCE: the [`ConsequenceFctSystems::Read`] set is placed
/// inside the [`PresenterSystems::Overlay`] stage (GTW-623 — the FCT palette draws over the
/// composed scene) and ordered `.after(animate_floating_text)` (GTW-793 — the reader's
/// [`FctSlotAllocator`] query must count pops AFTER the despawn system has flushed this frame's
/// expiries, the ordering convention `bevy-traps.md` #3 / [`FctSlotAllocator`] requires).
///
/// Called once by `TopDownRendererPlugin::build` before the per-family
/// [`ConsequenceFctAppExt::add_consequence_fct`] lines.
pub fn register_consequence_fct_core(app: &mut App) {
    app.configure_sets(
        Update,
        ConsequenceFctSystems::Read
            .in_set(PresenterSystems::Overlay)
            .after(animate_floating_text),
    );
}

/// The per-family registrar (GTW-572 C4): `app.add_consequence_fct::<Family>()` is the ONE
/// registration line a consequence family needs.
pub trait ConsequenceFctAppExt {
    /// Register family `C`'s generic stacked-pop reader with today's gate shape —
    /// `BattleInProgress` (pops belong to a live battle) + BOTH message buffers + the
    /// hot-reloadable [`FxTuning`] the pop lifetime/rise read.
    ///
    /// Both buffers are named, and each for its own reason:
    ///
    /// - `Messages<Played<C::Signal>>` is the buffer the reader ACTUALLY drains, so it is
    ///   the one param validation needs — a [`MessageReader`] whose buffer is absent fails
    ///   validation, and Bevy 0.19 routes that to the global error handler, which panics by
    ///   default (`bevy-traps.md` #1 / #4). `register_played_messages` registers it for
    ///   every family the cursor emits today, but nothing in the `C: ConsequenceFct` bound
    ///   requires that, so the gate says it rather than relying on registration order.
    /// - `Messages<C::Signal>` is the SIM buffer, kept so the inertness convention holds: a
    ///   presenter-only headless harness that registers no sim buffers keeps every family
    ///   reader inert, which the registrar-contract test relies on.
    ///
    /// NEVER calls `add_message` for the sim buffer: in a live battle the sim's plugins
    /// register it.
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
                        .and_then(resource_exists::<Messages<Played<C::Signal>>>)
                        .and_then(resource_exists::<Messages<C::Signal>>)
                        .and_then(resource_exists::<FxTuning>),
                ),
        );
        self
    }
}
