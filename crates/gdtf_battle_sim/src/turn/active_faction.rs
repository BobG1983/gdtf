//! The [`ActiveFaction`] battle-lifetime resource — whose team is currently acting
//! (GTW-309).

use bevy::prelude::{Deref, Resource};

use crate::ganger::Faction;

/// The number of teams in a battle — gang `0` and gang `1` (the two sides). The turn
/// cycle alternates between exactly these two.
const TEAM_COUNT: u8 = 2;

/// The **active faction** — which gang's turn it currently is (GTW-309).
///
/// A named newtype [`Resource`] over the [`Faction`] gang index (no-bare-types: it
/// carries a domain value — whose turn it is — so it is a real named newtype over
/// [`Faction`], NOT the bare [`Faction`]). The derived [`Deref`] reads the inner
/// [`Faction`] back; the inner field is PRIVATE, mutated only through [`ActiveFaction::advance`].
///
/// **Lifetime tracks [`BattleInProgress`](crate::battle::BattleInProgress):** the setup
/// system inserts it (seeded to the [`PlayerFaction`](crate::battle::PlayerFaction), so
/// the player acts first) on the same successful-setup `Ok` path that inserts
/// [`BattleInProgress`](crate::battle::BattleInProgress), and the teardown system removes
/// it alongside — so it is present for exactly the battle-active window. The
/// [`dispatch_end_turn`](crate::turn::dispatch_end_turn) system therefore reads its
/// `ResMut<ActiveFaction>` panic-free only because it is guarded on
/// [`resource_exists`](bevy::prelude::resource_exists)`::<ActiveFaction>` (`bevy-traps.md`
/// #1).
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActiveFaction(Faction);

impl ActiveFaction {
    /// Build the active-faction resource from the gang whose turn it is.
    ///
    /// The public constructor (house style) so the setup can seed `ActiveFaction` from the
    /// [`PlayerFaction`](crate::battle::PlayerFaction) (the player acts first) without
    /// reaching the private field.
    #[must_use]
    pub const fn new(faction: Faction) -> Self {
        Self(faction)
    }

    /// Advance the turn to the OTHER team — cycle gang `0` ⇄ `1`.
    ///
    /// The two-team turn alternation: the next gang index is `(current + 1) % 2`, so a
    /// turn ending on gang `0` advances to gang `1` and vice versa. Mutates the private
    /// inner [`Faction`] in place (the only mutation path — the inner stays private per the
    /// no-bare-types rule). A battle has exactly two sides this slice; a many-team cycle is
    /// a future refinement.
    pub fn advance(&mut self) {
        let next = (*self.0).wrapping_add(1) % TEAM_COUNT;
        self.0 = Faction::new(next);
    }
}
