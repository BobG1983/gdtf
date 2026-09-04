//! Right-panel grid span input and default theme seeding.

use bevy::prelude::*;
use gdtf_battle_sim::level::{ThemeUuid, UuidThemeRegistry};

use crate::session::MapEditorSession;

/// Grid width/height/levels value entered in the right panel.
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct GridSpanInput(u8);

impl GridSpanInput {
    /// Build from a cell count.
    #[must_use]
    pub const fn new(cells: u8) -> Self {
        Self(cells)
    }
}

impl core::fmt::Display for GridSpanInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::str::FromStr for GridSpanInput {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u8>().map(Self)
    }
}

pub(crate) fn seed_default_theme(
    themes: Option<Res<UuidThemeRegistry>>,
    session: Option<ResMut<MapEditorSession>>,
) {
    let (Some(themes), Some(mut session)) = (themes, session) else {
        return;
    };
    if !*session.theme().is_nil() {
        return;
    }
    let mut entries: Vec<(String, ThemeUuid)> = themes
        .defs()
        .map(|(key, def)| ((*def.display_name).clone(), *key))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let Some((_, theme)) = entries.into_iter().next() else {
        return;
    };
    let default_floor = themes.default_floor(&theme);
    session.select_theme(theme, default_floor);
}
