//! The per-ganger **inflicted-wound record** — the queryable list of the
//! individual wounds a ganger has taken (GTW-279).
//!
//! The sim tracks the life pool as a COUNT ([`Wounds`](crate::ganger::Wounds) —
//! the small `Wounds ≤ 0 → Dead` pool); this is its **presentation companion**: a
//! per-ganger LIST of the individual wounds the ganger has actually taken, each
//! carrying the rolled severity TIER ([`Severity`](crate::severity::Severity)) and
//! the struck BODY PART ([`BodyPart`](crate::armor::BodyPart)). The GTW-278
//! status / hover panels read it to render the wound-name list (e.g. "Minor — Left
//! Arm"); GTW-279 is the child sim prereq that BLOCKS that view slice.
//!
//! It is **additive to and consistent with** the [`Wounds`](crate::ganger::Wounds)
//! pool — it records, it does not replace: the pool's meaning is unchanged (the
//! count that gates Dead), and the list grows by one entry each time a wound
//! actually registers (the per-tier mapping below). The sim **never reads it back
//! for combat math** — it is a record for the view, not a balance input.
//!
//! ## The wound-to-pool mapping (AC3)
//!
//! The append site is the E3.6 wound-application fold ([`apply_hit`](crate::apply_hit::apply_hit)),
//! in the SAME place the [`Wounds`](crate::ganger::Wounds) pool is spent. A graze
//! ([`Severity::None`](crate::severity::Severity::None)) is "HP loss only — **no
//! Wound** is spent" (`docs/combat/resolution.md` §6), so it does NOT spend the
//! pool and so records **no** entry. Every **non-`None`** tier (Minor / Major /
//! Critical / Fatal) does register a wound on the pool (Minor / Major / Critical
//! each spend their tunable cost; Fatal empties the pool), so each appends exactly
//! one [`InflictedWound`] — the closest faithful 1:1 "one registered wound = one
//! list entry" mapping (the per-tier Wounds *cost* is not 1:1 with the *count* of
//! wounds — a Major spends 2 pool points but is still ONE injury — so the list
//! counts injuries, mirroring the pool's per-hit spend, not its magnitude).
//!
//! Reuses the EXISTING sim newtypes — [`Severity`](crate::severity::Severity) (the
//! tier) and [`BodyPart`](crate::armor::BodyPart) (the hit location) — so the record
//! introduces no new bare enum / string (no-bare-types).

mod types;

#[cfg(test)]
mod test;

pub use types::{InflictedWound, InflictedWounds};
