use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use crate::metric::MAX_LEVELS;

pub const MAX_GRID_SPAN: u8 = 60;

/// inside a [`GridSize`] is always in `1..=MAX_GRID_SPAN`. `#[serde(transparent)]`
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct GridWidth(u8);

impl GridWidth {
            #[must_use]
    pub const fn new(cells: u8) -> Self {
        Self(cells)
    }
}

/// `#[serde(transparent)]` parses a bare RON scalar.
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct GridHeight(u8);

impl GridHeight {
            #[must_use]
    pub const fn new(cells: u8) -> Self {
        Self(cells)
    }
}

/// [`MAX_LEVELS`](crate::metric::MAX_LEVELS) in [`GridSize::new`]. `#[serde(transparent)]`
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct GridLevels(u8);

impl GridLevels {
                #[must_use]
    pub const fn new(levels: u8) -> Self {
        Self(levels)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridSizeError {
        Empty {
                width:  GridWidth,
                height: GridHeight,
                levels: GridLevels,
    },
        WidthOverMax(GridWidth),
        HeightOverMax(GridHeight),
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

/// `GridSizeDef` shape, `#[serde(into = "GridSizeDef")]`) so the editor prefab saver
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(try_from = "GridSizeDef", into = "GridSizeDef")]
pub struct GridSize {
        width:  GridWidth,
        height: GridHeight,
        levels: GridLevels,
}

impl GridSize {
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

        #[must_use]
    pub const fn width(&self) -> GridWidth {
        self.width
    }

        #[must_use]
    pub const fn height(&self) -> GridHeight {
        self.height
    }

        #[must_use]
    pub const fn levels(&self) -> GridLevels {
        self.levels
    }
}

impl Default for GridSize {
                    /// field is `#[serde(default)]`, so a situation `.ron` that omits `grid_size` (every
                                fn default() -> Self {
        Self {
            width:  GridWidth::new(MAX_GRID_SPAN),
            height: GridHeight::new(MAX_GRID_SPAN),
            levels: GridLevels::new(MAX_LEVELS),
        }
    }
}

/// A serde intermediate (`#[serde(try_from = "GridSizeDef", into = "GridSizeDef")]` on
#[derive(Deserialize, Serialize)]
pub struct GridSizeDef {
        width:  GridWidth,
        height: GridHeight,
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
