//! Find the workspace root by searching for a marker, and the assets tree under it.

mod root;

#[cfg(test)]
mod test;

pub use root::{workspace_assets_root, workspace_root, workspace_root_from};
