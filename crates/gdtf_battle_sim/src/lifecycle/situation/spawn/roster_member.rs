//! [`RosterMember`] — the GTW-744 reduced roster reference: WHICH gang member fights
//! and on WHICH side, with NO authored placement (procgen derives the cell).

use serde::Deserialize;

use crate::ganger::{Faction, GangName, GangerName};

/// One **roster member** the situation fields — a gang + member reference plus the side
/// it fights on, carrying NO placement of its own (GTW-744).
///
/// This is the reduced, procgen-first half of the deployment model: a `Situation` lists
/// its combatants as `RosterMember`s (pure `(gang, member, faction)` refs) and the
/// procgen deploy step ([`deploy_rosters`](crate::procgen::deploy_rosters)) derives each
/// member's spawn `(cell, level)` / facing / stance from the generated map's deployment
/// zones. Contrast [`PlacedGanger`](crate::situation::PlacedGanger), which carries the
/// full authored placement (`at` / `facing` / `stance` / …) — kept for back-compat so a
/// fixture may still author exact cells. Shipped content authors `rosters` (zero cells);
/// only tests/fixtures author `gangers` cells.
///
/// The [`faction`](RosterMember::faction) encodes the side: a member whose faction equals
/// the situation's [`player_faction`](crate::situation::Situation::player_faction) deploys
/// in the player zone, every other faction in the enemy zone (the two-sided deployment
/// model, matching the two opposing prefab zones the assembler produces).
///
/// `PartialEq` + `Eq` (the gang / member names + the fieldless [`Faction`] all have a
/// total `Eq`). `Clone` (owned `String`-backed names). Derives [`Deserialize`] so an
/// authored situation `.ron` names each member as a self-describing `(gang, member,
/// faction)` record — the value graph flows through the landed newtype serde derives
/// (render-free, pixel-free).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RosterMember {
    /// The **gang** this member's roster comes from — a [`GangName`] resolved against the
    /// [`GangRegistry`](crate::ganger::GangRegistry) at setup (after the deploy step turns
    /// this member into a [`PlacedGanger`](crate::situation::PlacedGanger)). Authored as a
    /// bare string ([`GangName`] is `#[serde(transparent)]`).
    pub gang:    GangName,
    /// The **member** within the gang this ganger IS — a [`GangerName`] resolved against
    /// the gang's roster at setup. Authored as a bare string ([`GangerName`] is
    /// `#[serde(transparent)]`).
    pub member:  GangerName,
    /// The side (faction) this member fights for in THIS battle — the deployment zone it
    /// lands in is chosen by whether this equals the situation's
    /// [`player_faction`](crate::situation::Situation::player_faction). Authored as a bare
    /// gang index ([`Faction`] is `#[serde(transparent)]`).
    pub faction: Faction,
}

impl RosterMember {
    /// Build a roster member from its gang / member refs and the side it fights on.
    #[must_use]
    pub const fn new(gang: GangName, member: GangerName, faction: Faction) -> Self {
        Self {
            gang,
            member,
            faction,
        }
    }
}
