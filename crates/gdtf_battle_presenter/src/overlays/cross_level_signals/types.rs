//! The cross-level badge DOMAIN types (GTW-596): the signed [`LevelDelta`], the
//! aggregated [`ThreatCount`], the per-kind [`CrossLevelBadgeKind`], and the
//! derived [`CrossLevelSignals`] resource itself.

use bevy::{platform::collections::HashMap, prelude::*};
use gdtf_battle_sim::{falls::StoreysFallen, prelude::Cell};

/// A signed cross-storey distance from the presenter's active view storey —
/// positive when the OTHER `(cell, level)` sits ABOVE the active storey, negative
/// when it sits BELOW. Never zero: a badge only exists for a genuine cross-level
/// fact (the same-storey case is drawn as ordinary scenery, never a badge).
///
/// A named domain newtype over `i8` (no-bare-types): distinct from a bare
/// [`Level`](gdtf_battle_sim::prelude::Level) index (unsigned, absolute) or a
/// [`StoreysFallen`] distance (unsigned, describes a FALL, not an above/below
/// relation). Private inner + derived [`Deref`] (house style) so the sign reads
/// straight through `*delta`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LevelDelta(i8);

impl LevelDelta {
    /// Build a signed storey delta (positive above the active storey, negative below).
    #[must_use]
    pub const fn new(delta: i8) -> Self {
        Self(delta)
    }

    /// Whether the other storey sits ABOVE the active storey (`ThreatAbove` /
    /// an ascending `ConnectorDelta`) rather than below.
    #[must_use]
    pub const fn is_above(self) -> bool {
        self.0 > 0
    }

    /// The UNSIGNED storey distance — the RESOLVED SPEC's "nearest level-delta
    /// first" Threat priority tie-break sorts by this.
    #[must_use]
    pub const fn magnitude(self) -> u8 {
        self.0.unsigned_abs()
    }
}

/// How many squad-VISIBLE enemies share ONE aggregated [`CrossLevelBadgeKind::Threat`]
/// badge's exact [`LevelDelta`] on a cell (RESOLVED SPEC, user ruling 2026-07-10:
/// "all visible enemies sharing the same level-delta on one cell collapse into
/// ONE badge ... with a small count pip appended when MORE THAN ONE enemy shares
/// that delta").
///
/// A named domain newtype over `u8` (no-bare-types): distinct from any other raw
/// tally in the sim. Private inner + derived [`Deref`]; always `>= 1` (a badge
/// only exists once at least one enemy contributes it — see [`ThreatCount::ONE`]).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ThreatCount(u8);

impl ThreatCount {
    /// The count after exactly one enemy — the aggregation fold's starting value.
    pub const ONE: Self = Self(1);

    /// One more enemy sharing this badge's delta — saturating (a threat count is
    /// display-only; it never needs to exceed `u8::MAX`).
    #[must_use]
    pub const fn incremented(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// One rendered cross-level tactical badge's KIND (GTW-596) — what it signals and
/// the typed magnitude it carries.
///
/// A named domain enum (no-bare-types: every variant carries typed magnitudes,
/// never a bare tuple). [`Threat`](Self::Threat) folds the contract's separate
/// `ThreatAbove` / `ThreatBelow` labels into ONE variant keyed by a signed
/// [`LevelDelta`] — the RESOLVED SPEC dedupes Threat badges by "distinct
/// level-delta per cell", and the sign already IS the above/below distinction, so
/// a second axis would only duplicate it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossLevelBadgeKind {
    /// One or more squad-VISIBLE enemies sharing this exact level-delta from the
    /// active storey (never leaked for an UNSEEN / EXPLORED-only enemy — the
    /// load-bearing fog-gating invariant).
    Threat {
        /// The signed storey delta to the enemy/enemies (positive above, negative below).
        delta: LevelDelta,
        /// How many enemies share this exact delta on this cell.
        count: ThreatCount,
    },
    /// A hole / ledge cell — the storeys a ganger stepping here would fall.
    DropDepth {
        /// The fall distance [`resolve_drop`](gdtf_battle_sim::falls::resolve_drop) resolves.
        storeys: StoreysFallen,
    },
    /// A stair / ladder endpoint — the signed level-delta to its OTHER endpoint.
    ConnectorDelta {
        /// The signed storey delta to the link's other endpoint (positive =
        /// ascend, negative = descend).
        delta: LevelDelta,
    },
}

/// The maximum number of badges [`CrossLevelSignals`] keeps per cell — the
/// RESOLVED SPEC's cap, counted across ALL kinds combined. Past this the lowest
/// priority candidates are SILENTLY dropped (no overflow glyph exists in the
/// ~6-glyph art budget).
///
/// A `const`, not a domain newtype — a fixed design ceiling (the `PEEK_LEAN`-class
/// geometry-constant carve-out, `.claude/rules/no-bare-types.md` clause 4), never
/// authored / tunable data.
pub const BADGE_CAP_PER_CELL: usize = 3;

/// The presenter-derived cross-level tactical-badge signal set (GTW-596) — for
/// every cell on the active storey with a cross-level fact worth surfacing, the
/// (already aggregated + capped) ordered list of badges to draw there.
///
/// A Bevy [`Resource`] recomputed each frame by
/// [`derive_cross_level_signals`](super::derive::derive_cross_level_signals) from
/// the SAME pure fog queries [`present_fog`](crate::present_fog) reads, then written
/// through [`DetectChangesMut::set_if_neq`](bevy::prelude::DetectChangesMut::set_if_neq)
/// so it is change-tick-quiet: the draw system
/// ([`draw_cross_level_signals`](super::draw::draw_cross_level_signals)) re-walks
/// its pool only when this resource's `is_changed()` reads true OR the active
/// storey itself changed (a badge's drawn Z-band is hard-cut to the active
/// storey, so a level switch redraws even on the rare frame where the two
/// storeys' derived sets are identical), so a steady-state frame with
/// nothing new to surface touches no pooled sprite. Keyed by [`Cell`] alone (not
/// `CellLevel`) — the resource always describes the CURRENT active storey;
/// badges never persist across a level switch, they are recomputed fresh for the
/// new storey.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct CrossLevelSignals {
    per_cell: HashMap<Cell, Vec<CrossLevelBadgeKind>>,
}

impl CrossLevelSignals {
    /// Build the signal set from its per-cell (already capped) badge lists — the
    /// aggregation pipeline's one constructor.
    #[must_use]
    pub(super) const fn build(per_cell: HashMap<Cell, Vec<CrossLevelBadgeKind>>) -> Self {
        Self { per_cell }
    }

    /// The (already capped, priority-ordered) badges to draw at `cell` — empty
    /// when the cell has no cross-level fact worth surfacing.
    #[must_use]
    pub fn badges_at(&self, cell: Cell) -> &[CrossLevelBadgeKind] {
        self.per_cell.get(&cell).map_or(&[], Vec::as_slice)
    }

    /// Every cell this signal set holds at least one badge for.
    pub fn cells(&self) -> impl Iterator<Item = Cell> + '_ {
        self.per_cell.keys().copied()
    }
}
