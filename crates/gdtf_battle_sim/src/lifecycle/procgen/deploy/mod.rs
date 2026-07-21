//! Roster **deployment** (GTW-744) — surfacing the assembler's two deployment
//! [`zone`]s and placing each [`RosterMember`](crate::situation::RosterMember) into its
//! zone deterministically ([`place`]).
//!
//! `mod.rs` is wiring-only; the zone types + anchor→facing mapping live in [`zone`], the
//! seeded placement logic in [`place`].

mod place;
mod zone;

pub use place::{Standable, deploy_rosters};
pub use zone::{DeploymentZone, DeploymentZones, facing_for_anchor};
