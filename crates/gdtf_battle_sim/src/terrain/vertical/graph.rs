//! Vertical-link graph built from situation-authored links.

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::Resource,
};

use crate::{
    metric::{CellLevel, MAX_LEVELS},
    situation::Situation,
    vertical::links::VerticalLink,
};

/// Why a vertical link failed validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidVerticalLink {
    /// Either end is outside the valid level range.
    LevelOutOfRange {
        /// The bad link.
        link: VerticalLink,
    },
    /// Either end is not an authored cell.
    DanglingCell {
        /// The bad link.
        link: VerticalLink,
    },
    /// Both ends share the same Z.
    SameLevel {
        /// The bad link.
        link: VerticalLink,
    },
}

/// Indexed graph of vertical links for pathfinding.
#[derive(Resource, Debug, Clone, Default)]
pub struct VerticalLinkGraph {
    links: Vec<VerticalLink>,
    by_origin: HashMap<CellLevel, Vec<usize>>,
}

impl VerticalLinkGraph {
    /// Links reachable from this origin (including reverse for bidirectional).
    pub fn links_from(&self, origin: &CellLevel) -> impl Iterator<Item = &VerticalLink> {
        self.by_origin
            .get(origin)
            .into_iter()
            .flatten()
            .filter_map(|&i| self.links.get(i))
    }

    /// All links in the graph.
    pub fn links(&self) -> impl Iterator<Item = &VerticalLink> {
        self.links.iter()
    }

    /// Number of links.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.links.len()
    }

    /// Whether the graph has no links.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}

/// Build the vertical-link graph from situation-authored links.
///
/// # Errors
///
/// Returns [`InvalidVerticalLink::LevelOutOfRange`] if either end is outside valid levels,
/// [`InvalidVerticalLink::DanglingCell`] if an end is not an authored cell,
/// or [`InvalidVerticalLink::SameLevel`] if both ends share a Z.
pub fn build_vertical_link_graph(
    situation: &Situation,
) -> Result<VerticalLinkGraph, InvalidVerticalLink> {
    let authored: HashSet<CellLevel> = situation.authored_cells().collect();

    let mut graph = VerticalLinkGraph::default();

    for &link in &situation.vertical_links {
        validate_link(&link, &authored)?;

        let index = graph.links.len();
        graph.links.push(link);
        graph.by_origin.entry(link.from).or_default().push(index);
        if !*link.kind.is_one_way() {
            graph.by_origin.entry(link.to).or_default().push(index);
        }
    }

    Ok(graph)
}

fn validate_link(
    link: &VerticalLink,
    authored: &HashSet<CellLevel>,
) -> Result<(), InvalidVerticalLink> {
    let max_level = i32::from(MAX_LEVELS);
    let in_range = |key: &CellLevel| key.z >= 0 && key.z < max_level;

    if !in_range(&link.from) || !in_range(&link.to) {
        return Err(InvalidVerticalLink::LevelOutOfRange { link: *link });
    }
    if !authored.contains(&link.from) || !authored.contains(&link.to) {
        return Err(InvalidVerticalLink::DanglingCell { link: *link });
    }
    if link.from.z == link.to.z {
        return Err(InvalidVerticalLink::SameLevel { link: *link });
    }
    Ok(())
}
