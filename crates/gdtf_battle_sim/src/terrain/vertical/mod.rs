//! Vertical links: authored stairs and ladders between storeys.

mod graph;
mod links;
mod traverse;

#[cfg(test)]
mod test;

pub use graph::{InvalidVerticalLink, VerticalLinkGraph, build_vertical_link_graph};
pub use links::{LinkKind, OneWay, VerticalLink};
pub use traverse::traversable_links;
