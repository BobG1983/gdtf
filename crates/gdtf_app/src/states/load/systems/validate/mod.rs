//! per-edge check systems below walk the WHOLE authored content graph and
mod prefabs;
mod register;
mod situation;

pub(in crate::states::load) use register::add_content_validation;
