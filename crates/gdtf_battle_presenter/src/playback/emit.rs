//! [`Played<M>`] — the presenter-owned "this act is being SHOWN now" wrapper, and the
//! [`PlayedSignals`] writer bundle the cursor emits through (GTW-727 C16).

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    acts::{
        FireDeclaration, InjuryInflicted, MeleeResolved, MeleeStruck, MoveRejected,
        MovementOccurred, ReloadResult, ThrowResolved,
    },
    armor_wear::ArmorBroken,
    effects::{
        bleed::{BleedStarted, Bleeding},
        dot::DotAfflicted,
        fields::FieldAfflicted,
        on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    occupancy_sync::CoverDestroyed,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    turn::TurnStarted,
};

/// A sim fact `M` at the moment the presenter SHOWS it — as opposed to the moment the sim
/// produced it (GTW-727 C16).
///
/// The two are no longer the same instant. The sim resolves a whole reaction volley in one
/// tick; the presenter replays it over several seconds. A view system that must be paced
/// therefore reads `Played<M>` instead of `M` — a one-word change, since [`Deref`] means
/// the body reads the wrapped fact exactly as before and no trait impl on `M` has to move.
///
/// Written ONLY by [`advance_playback`](super::advance_playback), from what the sim already
/// recorded in its act log. The presenter never invents a fact and never re-derives one.
#[derive(Message, Deref, Debug, Clone)]
pub struct Played<M: Message + Clone>(M);

impl<M: Message + Clone> Played<M> {
    /// Wrap `fact` as being shown now.
    #[must_use]
    pub const fn new(fact: M) -> Self {
        Self(fact)
    }
}

/// Every `Played<M>` buffer the cursor writes, bundled into ONE [`SystemParam`] so
/// [`advance_playback`](super::advance_playback) declares one writer param instead of
/// twenty (the sim's `FireSignals` bundling precedent).
///
/// A transparent grouping of named output buffers — not itself a wrapped domain value.
///
/// One combat-log source is deliberately ABSENT: `ShotImpactResolved` is the presenter's
/// OWN per-impact signal, emitted by the impact animation when a bolt actually lands. Since
/// the bolt only spawns when the cursor plays its round, that signal is already at cursor
/// time by construction — and it could not be re-emitted here anyway, because at play time
/// the impact has not happened yet. Its combat-log forwarder therefore stays on the live
/// buffer, which is also what makes a shot's outcome line land exactly when the bolt does.
#[derive(SystemParam)]
pub struct PlayedSignals<'w> {
    /// A turn boundary reached the screen.
    pub(super) turn:          MessageWriter<'w, Played<TurnStarted>>,
    /// A walk step reached the screen.
    pub(super) step:          MessageWriter<'w, Played<MovementOccurred>>,
    /// A refused move reached the screen.
    pub(super) refusal:       MessageWriter<'w, Played<MoveRejected>>,
    /// A fire declaration reached the screen.
    pub(super) declaration:   MessageWriter<'w, Played<FireDeclaration>>,
    /// One round of a volley reached the screen — the bolt spawns off THIS.
    pub(super) round:         MessageWriter<'w, Played<ShotFired>>,
    /// A reload reached the screen.
    pub(super) reload:        MessageWriter<'w, Played<ReloadResult>>,
    /// An injury reached the screen.
    pub(super) injury:        MessageWriter<'w, Played<InjuryInflicted>>,
    /// A fall reached the screen.
    pub(super) fall:          MessageWriter<'w, Played<FallOccurred>>,
    /// A melee damage number reached the screen.
    pub(super) strike:        MessageWriter<'w, Played<MeleeStruck>>,
    /// A terminal death's consequence reached the screen.
    pub(super) death:         MessageWriter<'w, Played<OnDeathOccurred>>,
    /// A fresh suppression reached the screen.
    pub(super) suppression:   MessageWriter<'w, Played<SuppressionApplied>>,
    /// A broken armor plate reached the screen.
    pub(super) armor_broken:  MessageWriter<'w, Played<ArmorBroken>>,
    /// A started damage-over-time affliction reached the screen.
    pub(super) dot:           MessageWriter<'w, Played<DotAfflicted>>,
    /// A started field exposure reached the screen.
    pub(super) field:         MessageWriter<'w, Played<FieldAfflicted>>,
    /// A started bleed span reached the screen.
    pub(super) bleed_started: MessageWriter<'w, Played<BleedStarted>>,
    /// A per-round bleed drain reached the screen.
    pub(super) bleeding:      MessageWriter<'w, Played<Bleeding>>,
    /// A smashed piece of cover reached the screen.
    pub(super) cover:         MessageWriter<'w, Played<CoverDestroyed>>,
    /// A landed melee strike reached the screen.
    pub(super) melee_landed:  MessageWriter<'w, Played<MeleeResolved>>,
    /// A landed grenade reached the screen.
    pub(super) throw_landed:  MessageWriter<'w, Played<ThrowResolved>>,
}

/// Register every `Played<M>` buffer the cursor writes.
///
/// The presenter OWNS these buffers (it is their producer), so it registers them
/// unconditionally — the same "the producer registers its own buffer" rule the combat-log
/// and FX registrars follow for their own presenter-owned signals.
pub(super) fn register_played_messages(app: &mut App) {
    app.add_message::<Played<TurnStarted>>()
        .add_message::<Played<MovementOccurred>>()
        .add_message::<Played<MoveRejected>>()
        .add_message::<Played<FireDeclaration>>()
        .add_message::<Played<ShotFired>>()
        .add_message::<Played<ReloadResult>>()
        .add_message::<Played<InjuryInflicted>>()
        .add_message::<Played<FallOccurred>>()
        .add_message::<Played<MeleeStruck>>()
        .add_message::<Played<OnDeathOccurred>>()
        .add_message::<Played<SuppressionApplied>>()
        .add_message::<Played<ArmorBroken>>()
        .add_message::<Played<DotAfflicted>>()
        .add_message::<Played<FieldAfflicted>>()
        .add_message::<Played<BleedStarted>>()
        .add_message::<Played<Bleeding>>()
        .add_message::<Played<CoverDestroyed>>()
        .add_message::<Played<MeleeResolved>>()
        .add_message::<Played<ThrowResolved>>();
}
