//! Validated battle grid dimensions.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use crate::metric::MAX_LEVELS;

/// Maximum width or height in cells.
pub const MAX_GRID_SPAN: u8 = 60;

/// Grid width in cells (validated inside [`GridSize::new`]).
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct GridWidth(u8);

impl GridWidth {
    /// Wrap a width (unchecked; validated by [`GridSize::new`]).
    #[must_use]
    pub const fn new(cells: u8) -> Self {
        Self(cells)
    }
}

/// Grid height in cells.
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct GridHeight(u8);

impl GridHeight {
    /// Wrap a height (unchecked).
    #[must_use]
    pub const fn new(cells: u8) -> Self {
        Self(cells)
    }
}

/// Number of vertical levels in a grid.
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct GridLevels(u8);

impl GridLevels {
    /// Wrap a level count (unchecked).
    #[must_use]
    pub const fn new(levels: u8) -> Self {
        Self(levels)
    }
}

/// Why a grid size failed validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridSizeError {
    /// Any axis was zero.
    Empty {
        /// Requested width.
        width: GridWidth,
        /// Requested height.
        height: GridHeight,
        /// Requested levels.
        levels: GridLevels,
    },
    /// Width above [`MAX_GRID_SPAN`].
    WidthOverMax(GridWidth),
    /// Height above [`MAX_GRID_SPAN`].
    HeightOverMax(GridHeight),
    /// Levels above [`MAX_LEVELS`].
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

/// Validated battle grid dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(try_from = "GridSizeDef", into = "GridSizeDef")]
pub struct GridSize {
    width: GridWidth,
    height: GridHeight,
    levels: GridLevels,
}

impl GridSize {
    /// Build a grid size after range checks.
    ///
    /// # Errors
    ///
    /// Returns [`GridSizeError::Empty`] if any axis is zero,
    /// [`GridSizeError::WidthOverMax`] / [`HeightOverMax`] if width or height exceeds [`MAX_GRID_SPAN`],
    /// or [`GridSizeError::LevelsOverMax`] if levels exceed [`MAX_LEVELS`].
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

    /// Width in cells.
    #[must_use]
    pub const fn width(&self) -> GridWidth {
        self.width
    }

    /// Height in cells.
    #[must_use]
    pub const fn height(&self) -> GridHeight {
        self.height
    }

    /// Level count.
    #[must_use]
    pub const fn levels(&self) -> GridLevels {
        self.levels
    }
}

impl Default for GridSize {
    fn default() -> Self {
        Self {
            width: GridWidth::new(MAX_GRID_SPAN),
            height: GridHeight::new(MAX_GRID_SPAN),
            levels: GridLevels::new(MAX_LEVELS),
        }
    }
}

/// Serde intermediate for [`GridSize`].
#[derive(Deserialize, Serialize)]
pub struct GridSizeDef {
    /// Width in cells.
    width: GridWidth,
    /// Height in cells.
    height: GridHeight,
    /// Level count.
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
            width: size.width,
            height: size.height,
            levels: size.levels,
        }
    }
}
