//! The [`FallOccurred`] buffered message — the GTW-523 presenter-facing output signal
//! that a ganger fell when a slab was destroyed under it.

use bevy::prelude::{Entity, Message};

use crate::metric::Level;

/// A ganger **fell** — the presenter-facing output signal that a slab was destroyed under a
/// standing ganger and it dropped to the storey below (GTW-523 C6).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) emitted by
/// [`apply_falls`](super::apply_falls) once per fall (once per faller keyed to a destroyed
/// `(cell, level)`). It mirrors the melee output signal
/// [`MeleeResolved`](crate::acts::MeleeResolved) / the structural signals
/// ([`crate::occupancy_sync::SlabDestroyed`]): it carries ONLY what the presenter's
/// fall FX / log needs — WHO fell ([`ganger`](FallOccurred::ganger)), the storey it fell
/// FROM ([`from_level`](FallOccurred::from_level)), the storey it landed ON
/// ([`to_level`](FallOccurred::to_level)), and the [`storeys`](FallOccurred::storeys)
/// distance — never combat math. The presenter reads it through a
/// [`MessageReader`](bevy::prelude::MessageReader) (the one-way sim → presenter dep; the sim
/// never reads the presenter). The HP / wound / injury mutations the fall inflicts are
/// applied to the faller's components + the separate
/// [`InjuryInflicted`](crate::acts::InjuryInflicted) message and observed by the presenter
/// via change-detection + the existing wound/injury signals; this signal is the dedicated
/// *fell* moment.
///
/// The [`ganger`](FallOccurred::ganger) is a Bevy [`Entity`] handle (framework plumbing, the
/// only bare type the no-bare-types rule permits in a payload); the two storeys are typed
/// [`Level`] newtypes, and [`storeys`](FallOccurred::storeys) the typed distance.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FallOccurred {
    /// The ganger that fell — the entity whose [`Position`](crate::ganger::Position) was
    /// rewritten one-shot to the landing storey.
    pub ganger:     Entity,
    /// The storey the ganger fell FROM — its [`Position`](crate::ganger::Position) level
    /// before the fall (the level of the destroyed slab it was standing on).
    pub from_level: Level,
    /// The storey the ganger landed ON — the highest supported storey strictly below the
    /// start (ground always supports, else the first `Present` slab).
    pub to_level:   Level,
    /// How many storeys it fell — `from_level − to_level` (always `≥ 1`), the LINEAR damage
    /// scalar the falls system multiplies `per_storey_damage` by.
    pub storeys:    StoreysFallen,
}

impl FallOccurred {
    /// Build a fall-occurred signal for `ganger` that fell `storeys` from `from_level` down
    /// to `to_level`.
    #[must_use]
    pub const fn new(
        ganger: Entity,
        from_level: Level,
        to_level: Level,
        storeys: StoreysFallen,
    ) -> Self {
        Self {
            ganger,
            from_level,
            to_level,
            storeys,
        }
    }
}

/// The **number of storeys fallen** in one fall — `from_level − to_level`, always `≥ 1`
/// (GTW-523 C2 / C4).
///
/// A named newtype over `u8` (no-bare-types: a storey-distance is a domain value, distinct
/// from a [`Level`] index or any HP magnitude). It is the LINEAR scalar the §Falls damage
/// formula multiplies the [`PerStoreyDamage`](crate::tuning::PerStoreyDamage) leaf by
/// (`magnitude = per_storey_damage × storeys`). A `u8` because storey distance is bounded by
/// [`MAX_LEVELS`](crate::metric::MAX_LEVELS). Private inner + derived
/// [`Deref`](bevy::prelude::Deref) (house style).
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StoreysFallen(u8);

impl StoreysFallen {
    /// Build a storeys-fallen distance from its count (`≥ 1` for a real fall).
    #[must_use]
    pub const fn new(storeys: u8) -> Self {
        Self(storeys)
    }
}
