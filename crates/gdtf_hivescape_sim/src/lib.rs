//! Placeholder crate for the future **hivescape** sim (the render-free model).
//!
//! Scaffold only — carries no real logic yet. Exists so the workspace member is
//! present and lint-clean; real combat-sim logic lands under its own ticket.

/// Placeholder sum helper kept only so the scaffold crate has an item to compile.
///
/// Carries no domain meaning — it is replaced when the hivescape sim gets real
/// logic.
#[must_use]
pub const fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
