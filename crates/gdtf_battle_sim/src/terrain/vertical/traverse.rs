//! List cells reachable via vertical links from an origin.

use crate::{ganger::Tu, metric::CellLevel, tuning::LinkTu, vertical::VerticalLinkGraph};

/// Destination cells and TU cost for each vertical link leaving `origin`.
pub fn traversable_links(
    origin: CellLevel,
    graph: &VerticalLinkGraph,
    link_tu: LinkTu,
) -> impl Iterator<Item = (CellLevel, Tu)> {
    let cost = Tu::new(*link_tu);
    std::iter::once(origin)
        .flat_map(move |origin| {
            graph
                .links_from(&origin)
                .map(|link| {
                    if link.from == origin {
                        link.to
                    } else {
                        link.from
                    }
                })
                .collect::<Vec<_>>()
        })
        .map(move |to| (to, cost))
}
