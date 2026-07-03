//! The battlescape weapon cluster (GTW-275 / GTW-298, bottom-left): a themed `gdtf_ui` panel
//! group showing the selected player ganger's weapon and the relocated firemode / aim / stance
//! controls. The **Overall Weapon Panel** is a 2×2 grid — the Combined Weapon Panel (image
//! placeholder + name + magazine `"cur/max"` + a LIVE Reload button), the Firemode Panel (the
//! relocated mode toggles), the Item Panel (two disabled item buttons), and the Aim Panel (the
//! relocated aim toggle) — plus a SEPARATE Stance Panel to its right (the relocated stance
//! toggles).
//!
//! The Reload button is the VIEW end of the real, TU-costed reload act (the sim's
//! `dispatch_reload`): a press pushes [`ActIntent::Reload`](gdtf_battle_input::ActIntent::Reload)
//! onto the shared act-intent seam, draining to a
//! [`ReloadRequested`](gdtf_battle_sim::acts::ReloadRequested) for the selection. The relocated
//! firemode / aim / stance controls (GTW-298) keep their action-bar markers, so the existing
//! press → intent + active-mark systems drive them parent-agnostically.
//!
//! UI/view only: it reads the sim's weapon components + the input crate's `SelectedShooter`,
//! owns no combat rule, and its ONLY write is the act-intent seam. Mutate-in-place on
//! selection change ([[ui-mutate-not-respawn]]).

mod components;
mod plugin;
mod systems;

pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeWeaponPanelScenePlugin;

/// Test-support re-exports for this panel (GTW-569 one-hop ledger): the cluster ROOT
/// marker + the weapon-text / name / magazine / reload markers + the GTW-298 rework
/// structural markers (the Combined / Item / Aim grid cells, the image placeholder, the
/// disabled item buttons) the AC tests name through `crate::test_support` to assert the
/// authoritative hierarchy. `WeaponPanelRoot` is test-support-ONLY: after the GTW-275
/// layout overhaul the viewport insets the map by the bottom bar (NOT the weapon panel),
/// so no binary code reads it — the panel spawn/despawn reach the marker via the internal
/// `components::` path. The crate-root ledger (`src/test_support.rs`) re-exports these by
/// explicit name directly from here — no intermediate `mod.rs` climb. `pub(crate)` on the
/// module (not `pub`) because the parent chain is `pub(crate)`, so a `pub mod` here trips
/// the workspace `unreachable_pub = deny`.
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::components::{
        AimLabel, AimPanel, CombinedWeaponPanel, ReloadButton, WeaponContent, WeaponImage,
        WeaponItemButton, WeaponItemPanel, WeaponMagazineText, WeaponNameText, WeaponPanelRoot,
    };
}
