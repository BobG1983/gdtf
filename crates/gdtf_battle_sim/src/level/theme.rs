//! The **grid-size** dimension newtypes — the foundational typed data the map-editor
//! palette and the procgen assembly both read (GTW-409).
//!
//! [`GridSize`] is the chosen coarse-grid dimensions, every axis a named, validated,
//! private-inner newtype, constrained to the sim's coarse-grid maximum
//! ([`MAX_GRID_SPAN`] on x/y, [`MAX_LEVELS`](crate::metric::MAX_LEVELS) on z).

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use crate::metric::MAX_LEVELS;

/// The maximum span (in cells) of EITHER ground-plane axis of the coarse grid.
///
/// The coarse grid is 60×60×8 (`docs/combat/battle-space.md`): 60 cells on x AND on y,
/// `MAX_LEVELS` (= 8) storeys on z. `coords.rs` already names the z ceiling
/// ([`MAX_LEVELS`](crate::metric::MAX_LEVELS)); this names the x/y ceiling so a
/// [`GridSize`] can validate against a NAMED const rather than a scattered magic `60`.
/// A `u8` because 60 fits a tiny non-negative integer (the same reasoning
/// [`MAX_LEVELS`](crate::metric::MAX_LEVELS) is a `u8`).
pub const MAX_GRID_SPAN: u8 = 60;

/// The **width** of the coarse grid in cells (its x span).
///
/// A ground-plane span newtype over [`u8`] (no-bare-types rule 1: a dimension is a
/// domain value). Private inner + derived [`Deref`]; built through
/// [`GridSize::new`], which validates it against [`MAX_GRID_SPAN`] — so a `GridWidth`
/// inside a [`GridSize`] is always in `1..=MAX_GRID_SPAN`. `#[serde(transparent)]`
/// parses a bare RON scalar.
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct GridWidth(u8);

impl GridWidth {
    /// Build a grid width from its cell span. Validation against [`MAX_GRID_SPAN`]
    /// happens in [`GridSize::new`]; this is the bare wrap.
    #[must_use]
    pub const fn new(cells: u8) -> Self {
        Self(cells)
    }
}

/// The **height** of the coarse grid in cells (its y span).
///
/// A ground-plane span newtype over [`u8`] (no-bare-types rule 3: distinct from
/// [`GridWidth`] even over the same inner — a width is never a height). Private inner +
/// derived [`Deref`]; validated against [`MAX_GRID_SPAN`] in [`GridSize::new`].
/// `#[serde(transparent)]` parses a bare RON scalar.
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct GridHeight(u8);

impl GridHeight {
    /// Build a grid height from its cell span. Validation against [`MAX_GRID_SPAN`]
    /// happens in [`GridSize::new`]; this is the bare wrap.
    #[must_use]
    pub const fn new(cells: u8) -> Self {
        Self(cells)
    }
}

/// The number of **storeys** of the coarse grid (its z span).
///
/// A storey-count newtype over [`u8`] (no-bare-types rule 3: distinct from
/// [`GridWidth`]/[`GridHeight`] — a level count is not a ground span). Distinct also
/// from [`Level`](crate::metric::Level), which indexes ONE storey; this counts them.
/// Private inner + derived [`Deref`]; validated against
/// [`MAX_LEVELS`](crate::metric::MAX_LEVELS) in [`GridSize::new`]. `#[serde(transparent)]`
/// parses a bare RON scalar.
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct GridLevels(u8);

impl GridLevels {
    /// Build a storey count from its level span. Validation against
    /// [`MAX_LEVELS`](crate::metric::MAX_LEVELS) happens in [`GridSize::new`]; this is
    /// the bare wrap.
    #[must_use]
    pub const fn new(levels: u8) -> Self {
        Self(levels)
    }
}

/// Why a proposed [`GridSize`] was rejected — which axis exceeded its sim maximum, or
/// was zero (GTW-409).
///
/// The handled error of the validated [`GridSize::new`] constructor (no-bare-types:
/// the rejection reason is a domain value, not a bare `()`/`bool`; the no-panic
/// contract — `GridSize::new` returns this rather than `unwrap`/`panic`). Each variant
/// names the offending axis and the value seen, so a caller can log/surface exactly
/// what was wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridSizeError {
    /// An axis span was zero — a grid must be at least 1×1×1.
    Empty {
        /// The width seen.
        width:  GridWidth,
        /// The height seen.
        height: GridHeight,
        /// The level count seen.
        levels: GridLevels,
    },
    /// The width exceeded [`MAX_GRID_SPAN`].
    WidthOverMax(GridWidth),
    /// The height exceeded [`MAX_GRID_SPAN`].
    HeightOverMax(GridHeight),
    /// The level count exceeded [`MAX_LEVELS`](crate::metric::MAX_LEVELS).
    LevelsOverMax(GridLevels),
}

impl std::fmt::Display for GridSizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty {
                width,
                height,
                levels,
            } => write!(
                f,
                "grid size must be at least 1x1x1, got {}x{}x{}",
                **width, **height, **levels,
            ),
            Self::WidthOverMax(width) => {
                write!(f, "grid width {} exceeds the max {MAX_GRID_SPAN}", **width)
            }
            Self::HeightOverMax(height) => {
                write!(
                    f,
                    "grid height {} exceeds the max {MAX_GRID_SPAN}",
                    **height
                )
            }
            Self::LevelsOverMax(levels) => {
                write!(
                    f,
                    "grid level count {} exceeds the max {MAX_LEVELS}",
                    **levels
                )
            }
        }
    }
}

impl std::error::Error for GridSizeError {}

/// The chosen **coarse-grid dimensions** — a width × height × storey-count, each axis
/// constrained to its sim maximum (GTW-409).
///
/// The DECIDED shape (GTW-409 logged choice): a small struct of three named, validated,
/// private-inner dimension newtypes ([`GridWidth`] / [`GridHeight`] / [`GridLevels`]),
/// NOT a single newtype over a glam vector — because the three axes have DIFFERENT
/// ceilings (x/y bound by [`MAX_GRID_SPAN`], z by [`MAX_LEVELS`](crate::metric::MAX_LEVELS)),
/// so each deserves its own named type and its own validation rather than three lanes
/// of one `UVec3`. The struct fields are public to READ (the inner newtypes keep their
/// own private inners per no-bare-types rule 5), but the whole value is only
/// CONSTRUCTED through [`GridSize::new`], which enforces the bounds.
///
/// Built only through the validated [`GridSize::new`] (a handled [`Result`] — never a
/// panic/unwrap/expect), so an over-max or empty grid can never exist. Derives
/// [`Deserialize`] so a future authored situation (GTW-414) can carry it; the
/// deserialized value still flows through [`GridSize::new`] via the `GridSizeDef`
/// serde intermediate, so the bounds hold across deserialization too (the
/// [`CellDef`](crate::metric::CellDef) precedent). Derives [`Serialize`] (via the SAME
/// `GridSizeDef` shape, `#[serde(into = "GridSizeDef")]`) so the editor prefab saver
/// (GTW-432) writes a grid size that round-trips byte-for-byte back through the
/// `try_from` reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(try_from = "GridSizeDef", into = "GridSizeDef")]
pub struct GridSize {
    /// The grid's x span in cells (validated `1..=`[`MAX_GRID_SPAN`]).
    width:  GridWidth,
    /// The grid's y span in cells (validated `1..=`[`MAX_GRID_SPAN`]).
    height: GridHeight,
    /// The grid's storey count (validated `1..=`[`MAX_LEVELS`](crate::metric::MAX_LEVELS)).
    levels: GridLevels,
}

impl GridSize {
    /// Build a validated grid size, or a [`GridSizeError`] naming the offending axis.
    ///
    /// Validates every axis against its sim maximum — x/y against [`MAX_GRID_SPAN`], z
    /// against [`MAX_LEVELS`](crate::metric::MAX_LEVELS) — and rejects a zero span on
    /// any axis. Returns a handled [`Result`] (the no-panic contract: NO
    /// `unwrap`/`expect`/`panic`); a caller clamps or surfaces the error itself.
    ///
    /// # Errors
    ///
    /// [`GridSizeError::Empty`] if any axis is zero; [`GridSizeError::WidthOverMax`] /
    /// [`GridSizeError::HeightOverMax`] / [`GridSizeError::LevelsOverMax`] if that axis
    /// exceeds its sim maximum.
    pub fn new(
        width: GridWidth,
        height: GridHeight,
        levels: GridLevels,
    ) -> Result<Self, GridSizeError> {
        if *width == 0 || *height == 0 || *levels == 0 {
            return Err(GridSizeError::Empty {
                width,
                height,
                levels,
            });
        }
        if *width > MAX_GRID_SPAN {
            return Err(GridSizeError::WidthOverMax(width));
        }
        if *height > MAX_GRID_SPAN {
            return Err(GridSizeError::HeightOverMax(height));
        }
        if *levels > MAX_LEVELS {
            return Err(GridSizeError::LevelsOverMax(levels));
        }
        Ok(Self {
            width,
            height,
            levels,
        })
    }

    /// The grid's x span in cells.
    #[must_use]
    pub const fn width(&self) -> GridWidth {
        self.width
    }

    /// The grid's y span in cells.
    #[must_use]
    pub const fn height(&self) -> GridHeight {
        self.height
    }

    /// The grid's storey count.
    #[must_use]
    pub const fn levels(&self) -> GridLevels {
        self.levels
    }
}

impl Default for GridSize {
    /// The default coarse grid — the FULL documented `60×60×8` extent
    /// (`docs/combat/battle-space.md`: 60 cells on x AND y, [`MAX_LEVELS`] storeys on z).
    ///
    /// The chosen default (GTW-414): the [`Situation::grid_size`](crate::situation::Situation)
    /// field is `#[serde(default)]`, so a situation `.ron` that omits `grid_size` (every
    /// pre-GTW-414 file, incl. the shipped `skirmish.ron`) gets the full sim extent — the
    /// widest valid grid, the safe superset of any cell a pre-GTW-414 file authored (its
    /// gangers / cover / slabs already place within `1..=`[`MAX_GRID_SPAN`] /
    /// `1..=`[`MAX_LEVELS`]). Every axis is at its in-bounds maximum, so the value is
    /// always valid (it equals the bound [`GridSize::new`] validates against). A direct
    /// struct literal (not `GridSize::new`) so the impl is `const` — the magnitudes are
    /// the fixed sim maxima, never re-validated.
    fn default() -> Self {
        Self {
            width:  GridWidth::new(MAX_GRID_SPAN),
            height: GridHeight::new(MAX_GRID_SPAN),
            levels: GridLevels::new(MAX_LEVELS),
        }
    }
}

/// The authored RON shape a [`GridSize`] deserializes from — its three axis spans as a
/// named triple, routed through the validating [`GridSize::new`] (GTW-409).
///
/// A serde intermediate (`#[serde(try_from = "GridSizeDef", into = "GridSizeDef")]` on
/// [`GridSize`]) so an authored value writes `(width: .., height: .., levels: ..)` and
/// the value flows through the bounds-checked constructor on read AND back out through
/// the same shape on write — keeping every axis newtype's inner private and the
/// over-max/empty invariant intact across (de)serialization (the
/// [`CellDef`](crate::metric::CellDef) precedent, with `try_from` because the read
/// conversion is fallible; the write direction is infallible — a valid `GridSize`
/// always produces a valid `GridSizeDef`).
#[derive(Deserialize, Serialize)]
pub struct GridSizeDef {
    /// The grid's x span in cells.
    width:  GridWidth,
    /// The grid's y span in cells.
    height: GridHeight,
    /// The grid's storey count.
    levels: GridLevels,
}

impl TryFrom<GridSizeDef> for GridSize {
    type Error = GridSizeError;

    fn try_from(def: GridSizeDef) -> Result<Self, Self::Error> {
        Self::new(def.width, def.height, def.levels)
    }
}

impl From<GridSize> for GridSizeDef {
    fn from(size: GridSize) -> Self {
        Self {
            width:  size.width,
            height: size.height,
            levels: size.levels,
        }
    }
}
