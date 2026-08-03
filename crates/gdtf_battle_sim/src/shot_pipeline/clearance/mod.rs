mod band;
#[cfg(test)]
mod test;

pub use band::{
    Clearance, lower_band, round_band_for_cell, round_band_fraction, round_clears_occupant,
    silhouette_band,
};
