//! Enter/exit-emplacement act requests — [`EnterEmplacementRequested`] and
//! [`ExitEmplacementRequested`].

use bevy::prelude::{Entity, Message};

/// An **enter-emplacement** act was requested — `actor` mans the adjacent VACANT `emplacement`
/// (GTW-543, child GTW-41c).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying the acting
/// ganger [`Entity`] + the emplacement-piece [`Entity`]. The player-only contextual Enter button
/// writes this from the input seam when the selected ganger is 8-adjacent to a VACANT emplacement.
/// [`dispatch_enter_emplacement`](crate::acts::enter_emplacement::dispatch_enter_emplacement) drains it
/// and RE-GATES in the sim (the input layer's offer is advisory, never authoritative): the actor
/// exists + can afford the [`EnterEmplacementTu`](crate::tuning::EnterEmplacementTu) leaf, and the
/// `emplacement` entity is [`EmplacementState::Vacant`](crate::terrain::emplacement::EmplacementState)
/// and 8-adjacent to the actor. On pass it spends the
/// [`EnterEmplacementTu`](crate::tuning::EnterEmplacementTu) leaf off the actor and writes a
/// [`SetEmplacement::occupy`](crate::terrain::emplacement::SetEmplacement::occupy) — REUSING the
/// GTW-543 toggle mechanism verbatim (it never flips
/// [`EmplacementState`](crate::terrain::emplacement::EmplacementState) directly). The occupy then
/// forces the occupant's cover band + spawns the mounted gun through
/// [`apply_emplacement_toggle`](crate::terrain::emplacement::apply_emplacement_toggle).
///
/// The `actor` / `emplacement` are Bevy [`Entity`] handles — framework plumbing, the only bare
/// type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnterEmplacementRequested {
    /// The acting ganger manning the emplacement — the
    /// [`EnterEmplacementTu`](crate::tuning::EnterEmplacementTu) leaf is spent off its TU pool,
    /// and it is the 8-adjacency reference for the gate.
    pub actor:       Entity,
    /// The weapon-emplacement terrain piece to man — gated VACANT + 8-adjacent, then occupied
    /// through the shared GTW-543 [`SetEmplacement`](crate::terrain::emplacement::SetEmplacement)
    /// mechanism.
    pub emplacement: Entity,
}

impl EnterEmplacementRequested {
    /// Build an enter-emplacement request for `actor` manning `emplacement`.
    #[must_use]
    pub const fn new(actor: Entity, emplacement: Entity) -> Self {
        Self { actor, emplacement }
    }
}

/// An **exit-emplacement** act was requested — `actor` dismounts the `emplacement` it is manning
/// (GTW-543, child GTW-41c).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying the acting
/// ganger [`Entity`] + the emplacement-piece [`Entity`]. The player-only contextual Exit button
/// writes this from the input seam when the selected ganger IS the emplacement's occupant. Exit is
/// a SEPARATE TU-costed context action — there is NO force-eject (a ganger leaves the mount only
/// by spending [`ExitEmplacementTu`](crate::tuning::ExitEmplacementTu)).
/// [`dispatch_exit_emplacement`](crate::acts::enter_emplacement::dispatch_exit_emplacement) drains it and
/// RE-GATES: the `emplacement`'s
/// [`EmplacementOccupant`](crate::terrain::emplacement::EmplacementOccupant) IS the `actor` and the
/// actor can afford the [`ExitEmplacementTu`](crate::tuning::ExitEmplacementTu) leaf. On pass it
/// spends that leaf off the actor and writes a
/// [`SetEmplacement::vacate`](crate::terrain::emplacement::SetEmplacement::vacate) — the toggle
/// then restores the occupant's stance band + despawns the mounted gun.
///
/// The `actor` / `emplacement` are Bevy [`Entity`] handles — framework plumbing, the only bare
/// type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExitEmplacementRequested {
    /// The acting ganger dismounting — the [`ExitEmplacementTu`](crate::tuning::ExitEmplacementTu)
    /// leaf is spent off its TU pool, and it must BE the emplacement's recorded occupant.
    pub actor:       Entity,
    /// The weapon-emplacement terrain piece to dismount — gated so its recorded occupant IS the
    /// actor, then vacated through the shared GTW-543
    /// [`SetEmplacement`](crate::terrain::emplacement::SetEmplacement) mechanism.
    pub emplacement: Entity,
}

impl ExitEmplacementRequested {
    /// Build an exit-emplacement request for `actor` dismounting `emplacement`.
    #[must_use]
    pub const fn new(actor: Entity, emplacement: Entity) -> Self {
        Self { actor, emplacement }
    }
}
