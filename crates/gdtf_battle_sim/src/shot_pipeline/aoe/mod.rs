//! Area-of-effect footprints for blast, cone, and line hits.

mod resolve;

#[cfg(test)]
mod test;

pub use resolve::aoe_affected;
