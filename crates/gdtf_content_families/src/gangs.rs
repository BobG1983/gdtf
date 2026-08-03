//! Gangs content family.

use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::ganger::{GangName, GangRegistry, GangRoster};

/// Loads `*.gang.ron` files into [`GangRegistry`].
pub struct GangsFamily;

impl ContentFamily for GangsFamily {
    type Spec = GangRoster;
    type Registry = GangRegistry;

    const EXTENSION: &'static str = "gang.ron";
    const FOLDER: &'static str = "content/gangs";

    fn insert_member(
        registry: &mut GangRegistry,
        stem: Option<ContentFileStem>,
        roster: &GangRoster,
    ) {
        let Some(stem) = stem else { return };
        registry.insert(GangName::new(stem.into_inner()), roster.clone());
    }
}
