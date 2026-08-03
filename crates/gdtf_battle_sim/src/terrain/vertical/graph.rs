use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::Resource,
};

use crate::{
    metric::{CellLevel, MAX_LEVELS},
    situation::Situation,
    vertical::links::VerticalLink,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidVerticalLink {
            LevelOutOfRange {
                link: VerticalLink,
    },
                DanglingCell {
                link: VerticalLink,
    },
            SameLevel {
                link: VerticalLink,
    },
}

#[derive(Resource, Debug, Clone, Default)]
pub struct VerticalLinkGraph {
        links:     Vec<VerticalLink>,
                by_origin: HashMap<CellLevel, Vec<usize>>,
}

impl VerticalLinkGraph {
                                    pub fn links_from(&self, origin: &CellLevel) -> impl Iterator<Item = &VerticalLink> {
        self.by_origin
            .get(origin)
            .into_iter()
            .flatten()
            .filter_map(|&i| self.links.get(i))
    }

                                        pub fn links(&self) -> impl Iterator<Item = &VerticalLink> {
        self.links.iter()
    }

            #[must_use]
    pub const fn len(&self) -> usize {
        self.links.len()
    }

        #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}

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
