//! Vertical-link graph: the model's authored **stair / ladder** links between
//! (`docs/combat/combat.md`: "gangers change storeys **only over authored
//! into the movement graph)"). The authored situation carries these stair/ladder
mod graph;
mod links;
mod traverse;

#[cfg(test)]
mod test;

pub use graph::{InvalidVerticalLink, VerticalLinkGraph, build_vertical_link_graph};
pub use links::{LinkKind, OneWay, VerticalLink};
pub use traverse::traversable_links;
