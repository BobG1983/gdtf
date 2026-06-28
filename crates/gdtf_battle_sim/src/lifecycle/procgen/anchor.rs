//! The **deployment anchors** + the strict-geometric-opposite map (GTW-424).
//!
//! An [`Anchor`] names one of eight corner/edge zones a spawn prefab can be placed at.
//! The PLAYER anchor is chosen from the four GTW-424 supports
//! ([`Anchor::PLAYER_ANCHORS`] — `TopRight` / `BottomRight` / `RightMiddle` /
//! `BottomMiddle`) via one [`ProcgenRng`](crate::rng::ProcgenRng) draw ([`Anchor::choose`]).
//! The ENEMY anchor is the STRICT geometric [`opposite`](Anchor::opposite) of the player
//! anchor (OQ-2 RULED): no RNG draw decides the enemy side, so fairness is structural —
//! `TopRight <-> BottomLeft`, `BottomRight <-> TopLeft`, `RightMiddle <-> LeftMiddle`,
//! `BottomMiddle <-> TopMiddle`.

use crate::rng::ProcgenRng;

/// One placement **anchor** — a corner or edge-middle zone a spawn prefab is placed
/// flush against (GTW-424 C1/C2).
///
/// A named domain enum (no-bare-types: a deployment zone is a domain value, not a bare
/// index/string). The full eight zones exist so the strict-opposite map
/// ([`opposite`](Anchor::opposite)) is a true geometric involution over the board — the
/// player anchor is restricted to the four [`Anchor::PLAYER_ANCHORS`] the ticket names,
/// but each of those mirrors to a DIFFERENT zone (`TopRight -> BottomLeft` etc.), so the
/// opposite zones (`BottomLeft` / `TopLeft` / `LeftMiddle` / `TopMiddle`) must also be
/// nameable. Derives [`Copy`]/[`Eq`]/[`Hash`] (a tiny copyable key the packer matches on).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anchor {
    /// The top-right corner (max x, max y).
    TopRight,
    /// The top-left corner (min x, max y).
    TopLeft,
    /// The bottom-right corner (max x, min y).
    BottomRight,
    /// The bottom-left corner (min x, min y).
    BottomLeft,
    /// The middle of the right edge (max x, centred on y).
    RightMiddle,
    /// The middle of the left edge (min x, centred on y).
    LeftMiddle,
    /// The middle of the top edge (centred on x, max y).
    TopMiddle,
    /// The middle of the bottom edge (centred on x, min y).
    BottomMiddle,
}

impl Anchor {
    /// The four anchors a PLAYER spawn may be placed at, in a FIXED order — the
    /// RNG-selection domain for the player anchor (C1) and the determinism anchor.
    ///
    /// Order is load-bearing for replay: [`Anchor::choose`] draws an index into THIS
    /// slice, so reordering it would re-map every seed. Treat it as a stable contract.
    /// These are exactly the four the ticket names; their opposites (`BottomLeft` /
    /// `TopLeft` / `LeftMiddle` / `TopMiddle`) are the enemy zones and are NOT drawn.
    pub const PLAYER_ANCHORS: [Self; 4] = [
        Self::TopRight,
        Self::BottomRight,
        Self::RightMiddle,
        Self::BottomMiddle,
    ];

    /// The STRICT geometric opposite of this anchor (OQ-2 RULED) — the enemy-spawn
    /// anchor, decided with ZERO RNG so fairness is structural.
    ///
    /// The exact mapping the ticket pins: `TopRight <-> BottomLeft`,
    /// `BottomRight <-> TopLeft`, `RightMiddle <-> LeftMiddle`,
    /// `BottomMiddle <-> TopMiddle`. It is a total involution over all eight zones
    /// (`a.opposite().opposite() == a`) — each zone maps to its mirror through the board
    /// centre, so a player spawn and its opposite enemy spawn are always on opposite
    /// sides of the board with no draw between them.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::TopRight => Self::BottomLeft,
            Self::BottomLeft => Self::TopRight,
            Self::BottomRight => Self::TopLeft,
            Self::TopLeft => Self::BottomRight,
            Self::RightMiddle => Self::LeftMiddle,
            Self::LeftMiddle => Self::RightMiddle,
            Self::BottomMiddle => Self::TopMiddle,
            Self::TopMiddle => Self::BottomMiddle,
        }
    }

    /// Deterministically choose a PLAYER anchor from [`Anchor::PLAYER_ANCHORS`] via one
    /// [`ProcgenRng`](crate::rng::ProcgenRng) draw (C1).
    ///
    /// The ONLY RNG draw in anchor selection (the enemy anchor is the zero-draw strict
    /// [`opposite`](Anchor::opposite), OQ-2). The draw is a single
    /// `random_range(0..PLAYER_ANCHORS.len())` index into the fixed slice, so the same
    /// seed always picks the same player anchor (the determinism contract).
    #[must_use]
    pub fn choose(rng: &mut ProcgenRng) -> Self {
        let index: usize = rng.random_range(0..Self::PLAYER_ANCHORS.len());
        // `index` is always in `0..PLAYER_ANCHORS.len()` by construction of the range, so
        // this can never be out of bounds — but we fall back to the first anchor rather
        // than index-panic, honouring the no-panic contract.
        Self::PLAYER_ANCHORS
            .get(index)
            .copied()
            .unwrap_or(Self::TopRight)
    }
}
