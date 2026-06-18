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

pub(in crate::scenes::running::game::battlescape) use plugin::GameBattleScapeWeaponPanelScenePlugin;

// Test-support-only re-export of the cluster ROOT marker + the weapon-text / name / magazine /
// reload markers + the GTW-298 rework structural markers (the Combined / Item / Aim grid cells,
// the image placeholder, the disabled item buttons), carried toward `crate::test_support` so the
// AC tests can name them and assert the authoritative hierarchy; gated so the binary build stays
// `unused`/`unreachable_pub`-clean.
//
// `WeaponPanelRoot` is test-support-ONLY: after the GTW-275 layout overhaul the viewport insets
// the map by the bottom bar (NOT the weapon panel), so no binary code reads this re-export — the
// panel spawn/despawn reach the marker via the internal `components::` path.
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{
        AimPanel, CombinedWeaponPanel, ReloadButton, WeaponContent, WeaponImage, WeaponItemButton,
        WeaponItemPanel, WeaponMagazineText, WeaponNameText, WeaponPanelRoot,
    };
}
