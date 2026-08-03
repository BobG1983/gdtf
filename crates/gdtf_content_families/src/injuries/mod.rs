mod build;
mod keying;
mod layout;
mod salvage;

pub use build::build_injury_data;
pub use layout::{
    INJURIES_FOLDER, INJURY_DEF_EXTENSION, INJURY_WEIGHTING_EXTENSION, WEIGHTING_SUBFOLDER,
    category_dir, weighting_context_infix,
};
pub use salvage::{begin_injuries_salvage, settle_injuries_salvage};
