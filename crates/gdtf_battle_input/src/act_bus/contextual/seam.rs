//! The GENERIC contextual-act seam (GTW-571): the [`ContextualAct`] descriptor trait,
//! the per-act buffered [`PendingContextualIntents`] queue, the per-act generic
//! [`drain_contextual_intents`] system, and the compile-time [`ContextualActAppExt`]
//! registrar.
//!
//! # The invariant (GTW-571, Q5)
//!
//! The documented single-drain invariant is: **per-act generic drains in one
//! explicitly-ordered `SystemSet`, same-frame semantics preserved.** Each contextual act's
//! presses are still BUFFERED (never applied inline) through one per-act write-point
//! ([`PendingContextualIntents::push`] — the [`PendingActIntent`](crate::PendingActIntent)
//! single-write-point shape, per act), and ONE generic drain per act
//! ([`drain_contextual_intents::<A>`](drain_contextual_intents)) is the only place that
//! queue takes effect. Every per-act drain runs in the ONE explicitly-ordered
//! [`ContextualActSystems::Drain`] set — configured ONCE by
//! [`GdtfBattleInputPlugin`](crate::GdtfBattleInputPlugin) inside
//! [`InputSystems::Gather`](crate::InputSystems) and
//! `.before(`[`dispatch_act_intents`](crate::dispatch_act_intents)`)` — so a press pushed
//! this update is drained this update (`bevy-traps.md` #3), the emitted `*Requested` is
//! consumed by the sim the SAME frame (the input band precedes `SimSystems::Simulate`),
//! and the drains hold a deterministic order against the classic intent drain.
//!
//! # Why a descriptor trait + registrar, not a runtime table
//!
//! Adding a contextual act is COMPILE-TIME generic registration —
//! `app.add_contextual_act::<A>()`, mirroring `add_message::<M>` /
//! `RonAssetAppExt` — never a runtime `Vec<Box<dyn Descriptor>>` table (GTW-571 P4).
//! Each act is one vertical module per crate layer (the dep direction forbids one
//! cross-crate module): the sim owns the `*Requested` type + its bespoke dispatch, this
//! crate owns the act's [`ContextualAct`] descriptor, and `gdtf_app`'s contextual panel
//! owns the button-side descriptor + offer scan. The registrars stitch the layers.

use bevy::{ecs::message::Message, prelude::*};
use gdtf_battle_sim::BattleInProgress;

use crate::{InputSystems, SelectedShooter, intent::dispatch_act_intents};

/// A CONTEXTUAL act's input-layer descriptor (GTW-571) — the compile-time contract the
/// generic seam is stamped over, one impl per act (one vertical act module per crate
/// layer, stitched by [`ContextualActAppExt::add_contextual_act`]).
///
/// A contextual act is a button-only act on an OFFERED target (Execute / Stabilize /
/// Melee / Shove / Open Door / Enter Emplacement / Exit Emplacement / Throw Grenade):
/// the `gdtf_app` panel offers a target, a press pushes it onto the act's
/// [`PendingContextualIntents`] queue, and the act's generic drain emits
/// [`request`](Self::request)`(actor, target)` for the [`SelectedShooter`] as the actor.
/// The descriptor carries what the eight acts actually VARY in at this layer — the
/// target payload type and the sim `*Requested` message — and deliberately carries **NO
/// keybind field**: contextual acts are button-only (GTW-571 Q8, ruled); revisit only
/// through play discovery. The typed [`Keybinds`](crate::Keybinds) serde struct is
/// untouched by this seam.
pub trait ContextualAct: Send + Sync + 'static {
    /// The offered TARGET payload a press carries — a raw [`Entity`] handle for the
    /// entity-targeted acts, a [`CellLevel`](gdtf_battle_sim::CellLevel) for the
    /// cell-targeted ones, or a per-act domain enum (the melee ganger-or-structure
    /// target). `Copy + PartialEq` so the queue and the panel's offer resource stay
    /// value-plumbing.
    type Target: Copy + PartialEq + core::fmt::Debug + Send + Sync + 'static;

    /// The sim `*Requested` message the act's drain emits — the per-act message TYPE
    /// stays (GTW-571 P9: no mega-enum); the sim's bespoke dispatch reads it.
    type Requested: Message;

    /// Build the act's `*Requested` for `actor` acting on `target`.
    ///
    /// The ONE act-specific mapping at this layer. The sim's own dispatch gate is the
    /// authoritative check (adjacency / faction / state / TU) — this layer's offer is
    /// advisory, so the drain emits unconditionally once an actor is selected.
    fn request(actor: Entity, target: Self::Target) -> Self::Requested;
}

/// The buffered per-act contextual-intent QUEUE — act `A`'s single write-point
/// (GTW-571 C3).
///
/// The per-act mirror of [`PendingActIntent`](crate::PendingActIntent): a named newtype
/// over a `Vec` of offered targets (no-bare-types: the collection-of-domain-values
/// carve-out), `init_resource`-d by [`ContextualActAppExt::add_contextual_act`] and
/// DRAINED every update by [`drain_contextual_intents::<A>`](drain_contextual_intents),
/// so a buffered press is acted on exactly once. The `gdtf_app` panel's per-act press
/// system is the only producer ([`push`](Self::push)); the act's generic drain is the
/// only consumer.
#[derive(Resource, Debug)]
pub struct PendingContextualIntents<A: ContextualAct>(Vec<A::Target>);

impl<A: ContextualAct> Default for PendingContextualIntents<A> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<A: ContextualAct> PendingContextualIntents<A> {
    /// Queue `target` to be drained by
    /// [`drain_contextual_intents::<A>`](drain_contextual_intents) next time it runs.
    ///
    /// The act's single write-point — the panel's press system pushes the target the
    /// press carried. Buffered (not applied inline) so the act's ONE generic drain is
    /// the only place a press takes effect (the Q5 invariant).
    pub fn push(&mut self, target: A::Target) {
        self.0.push(target);
    }

    /// Take and clear every queued target — the drain's read.
    ///
    /// Returns the buffered targets in push order and leaves the queue empty, so a
    /// press is acted on exactly once. `pub(crate)` — only the in-crate generic drain
    /// consumes the queue; external surfaces only [`push`](Self::push).
    pub(crate) fn drain(&mut self) -> Vec<A::Target> {
        core::mem::take(&mut self.0)
    }

    /// Whether the queue currently holds no targets (test/inspection helper).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The input system-ordering set holding EVERY per-act contextual drain (GTW-571 C3 —
/// the Q5 "one explicitly-ordered `SystemSet`").
///
/// Configured ONCE by [`GdtfBattleInputPlugin`](crate::GdtfBattleInputPlugin)
/// (`bevy-traps.md` #5 — `configure_sets` precedes `.in_set`):
/// `.in_set(`[`InputSystems::Gather`](crate::InputSystems)`)` (so every drained
/// `*Requested` is emitted BEFORE the sim's `SimSystems::Simulate` consumes it the same
/// frame) and `.before(`[`dispatch_act_intents`](crate::dispatch_act_intents)`)` (so the
/// contextual drains read the SAME start-of-update [`SelectedShooter`] the classic
/// drain's selection-mutating arms have not yet touched — an explicit edge, never an
/// ambiguous order, `bevy-traps.md` #3). The `gdtf_app` panel orders its per-act press
/// systems `.before` this set, preserving the same-frame press -> `*Requested`
/// guarantee. Intra-set order is free: each drain touches ONLY its own act's queue +
/// message buffer (pairwise-disjoint data).
///
/// A framework `SystemSet` label, not a domain value (the no-bare-types framework
/// carve-out — the `InputSystems` justification) — `pub` so the app layer and tests can
/// order against it.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextualActSystems {
    /// The band holding every per-act generic contextual drain.
    Drain,
}

/// **Drain** act `A`'s queued contextual intents — the per-act generic drain
/// (GTW-571 C3).
///
/// Drains [`PendingContextualIntents<A>`] every update (gated on `BattleInProgress` by
/// the registrar) and emits [`A::request`](ContextualAct::request)`(actor, target)` per
/// queued target, the actor resolved from the [`SelectedShooter`] — a no-op with no
/// selection (fail-closed, the shape every contextual arm always had). The sim's
/// bespoke dispatch gate is the authoritative check; this layer's offer is advisory.
///
/// Registered by [`ContextualActAppExt::add_contextual_act`] into the ONE
/// explicitly-ordered [`ContextualActSystems::Drain`] set (the Q5 invariant — see the
/// module doc). Param-only (`bevy-traps.md` #7): the act's queue, the read-only
/// selection, and the act's [`MessageWriter`] (`bevy-traps.md` #4 — buffered messages).
pub fn drain_contextual_intents<A: ContextualAct>(
    mut pending: ResMut<PendingContextualIntents<A>>,
    selected: Res<SelectedShooter>,
    mut requests: MessageWriter<A::Requested>,
) {
    for target in pending.drain() {
        let Some(actor) = **selected else { continue };
        requests.write(A::request(actor, target));
    }
}

/// Compile-time contextual-act REGISTRAR (GTW-571 C1) — the input layer's
/// one-line-per-act extension, mirroring [`App::add_message`] / `RonAssetAppExt`.
///
/// `app.add_contextual_act::<A>()` wires act `A`'s whole input-layer slice: the sim
/// `*Requested` buffer, the per-act pending queue, and the per-act generic drain in the
/// ONE explicitly-ordered [`ContextualActSystems::Drain`] set. NEVER a runtime
/// descriptor table (P4) — the act set is closed at compile time by the registration
/// lines in [`GdtfBattleInputPlugin`](crate::GdtfBattleInputPlugin).
pub trait ContextualActAppExt {
    /// Register contextual act `A` on the input layer: `add_message::<A::Requested>()`
    /// (IDEMPOTENT — coexists with the sim's own registration, `bevy-traps.md` #4),
    /// `init_resource::<PendingContextualIntents<A>>()`, and
    /// [`drain_contextual_intents::<A>`](drain_contextual_intents) in
    /// [`ContextualActSystems::Drain`], gated
    /// `run_if(resource_exists::<BattleInProgress>)` (inert pre-battle,
    /// `bevy-traps.md` #1).
    fn add_contextual_act<A: ContextualAct>(&mut self) -> &mut Self;
}

impl ContextualActAppExt for App {
    fn add_contextual_act<A: ContextualAct>(&mut self) -> &mut Self {
        self.add_message::<A::Requested>()
            .init_resource::<PendingContextualIntents<A>>()
            .add_systems(
                Update,
                drain_contextual_intents::<A>
                    .in_set(ContextualActSystems::Drain)
                    .run_if(resource_exists::<BattleInProgress>),
            )
    }
}

/// Configures the [`ContextualActSystems::Drain`] set's explicit ordering — ONCE, from
/// [`GdtfBattleInputPlugin`](crate::GdtfBattleInputPlugin)'s `build` (GTW-571 P5: the
/// registrar-owned `SystemSet` vocabulary is configured in one place).
///
/// The set joins [`InputSystems::Gather`](crate::InputSystems) (every drained
/// `*Requested` is emitted before `SimSystems::Simulate` consumes it the same update)
/// and runs `.before(`[`dispatch_act_intents`]`)` (the contextual drains read the
/// start-of-update selection the classic drain's selection-mutating arms have not yet
/// touched — an explicit edge, `bevy-traps.md` #3).
pub fn configure_contextual_act_drains(app: &mut App) {
    app.configure_sets(
        Update,
        ContextualActSystems::Drain
            .in_set(InputSystems::Gather)
            .before(dispatch_act_intents),
    );
}
