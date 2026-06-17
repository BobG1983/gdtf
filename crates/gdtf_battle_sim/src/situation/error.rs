//! The typed [`BattleSetupError`] — the no-panic setup-abort contract.

use crate::{vertical::InvalidVerticalLink, weapon::WeaponName};

/// The typed ways [`setup_battle`](crate::situation::setup_battle) can fail — the
/// no-panic setup-abort contract (GTW-257).
///
/// A named domain enum (no-bare-types: a setup failure is a domain value, not a
/// bare string / `()`), returned in the `Err` arm of
/// [`setup_battle`](crate::situation::setup_battle)'s [`Result`].
/// It subsumes the prior `Err(InvalidVerticalLink)` (now the
/// [`InvalidLink`](BattleSetupError::InvalidLink) variant) and adds the GTW-257
/// [`WeaponNotFound`](BattleSetupError::WeaponNotFound) variant for a
/// [`GangerSpawn::weapon`](crate::situation::GangerSpawn::weapon) key that no loaded
/// weapon file supplies. The caller
/// ([`setup_battle_on_request`](crate::battle::setup_battle_on_request)) matches on
/// it and fails closed (logs, no [`BattleReady`](crate::battle::BattleReady)) — it
/// NEVER panics / unwraps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleSetupError {
    /// An authored vertical link failed validation (level out of range, dangling
    /// endpoint, or same-storey) — the prior `Err(InvalidVerticalLink)`, now wrapped.
    InvalidLink(InvalidVerticalLink),
    /// A ganger's [`weapon`](crate::situation::GangerSpawn::weapon) key was not in the
    /// [`WeaponRegistry`](crate::weapon::WeaponRegistry) — no `assets/weapons/*.ron`
    /// with that filename stem loaded.
    WeaponNotFound {
        /// The unresolved weapon key (the missing file's stem).
        weapon: WeaponName,
    },
}

impl From<InvalidVerticalLink> for BattleSetupError {
    /// Lift a vertical-link validation failure into a setup error — so
    /// [`setup_battle`](crate::situation::setup_battle) can `?`-propagate
    /// [`build_vertical_link_graph`](crate::vertical::build_vertical_link_graph)'s
    /// error straight into its own [`BattleSetupError`] result.
    fn from(invalid: InvalidVerticalLink) -> Self {
        Self::InvalidLink(invalid)
    }
}
