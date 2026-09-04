//! Authoring-time content validation for the editor.
mod rearm;
mod register;

#[cfg(test)]
mod test;

pub(crate) use register::register_validation;
