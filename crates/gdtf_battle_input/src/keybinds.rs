//! The data-driven keybind table (GTW-225 / GTW-48 S8): the FIRST keybind `.ron`
//! in the repo and its loader.
//!
//! Every bound act names a key in a loose, per-line-commented
//! `assets/input/keybinds.ron`, loaded through the generic GTW-136
//! [`RonAsset<T>`](gdtf_assets::RonAsset) path (the `tile_roles` / `character_roles`
//! precedent) and resolved into a resident [`Keybinds`] resource. The systems read
//! the resolved [`KeyCode`] off that resource — there is NO hardcoded `KeyCode`
//! literal in any system.
//!
//! # Why an authored [`BoundKey`] vocabulary instead of a bare `KeyCode`
//!
//! Bevy's [`KeyCode`] only derives `serde::Deserialize` under its `serialize`
//! feature, which is NOT in the workspace's default Bevy feature set (enabling it
//! fans out a heavy recompile across the whole engine). So the authored leaf is a
//! named domain [`BoundKey`] enum — a small, explicit key vocabulary that
//! `Deserialize`s from a RON name and [resolves to](BoundKey::key_code) a
//! [`KeyCode`]. This is the no-bare-types-aligned shape (an authored key is a
//! named domain value, not a raw framework code) AND it keeps the binding
//! data-driven: the one [`BoundKey`]→[`KeyCode`] mapping is a typed translation,
//! and every system reads the resolved [`KeyCode`] from [`Keybinds`], never a
//! literal.

use bevy::prelude::*;
use gdtf_assets::RonAsset;
use serde::Deserialize;

/// The path of the loose keybind RON, relative to the asset source root.
const KEYBINDS_RON_PATH: &str = "input/keybinds.ron";

/// The authored key vocabulary — the named keys a [`Keybinds`] leaf can bind.
///
/// A domain enum (no-bare-types: an authored binding is a named key, not a raw
/// framework [`KeyCode`]) that `Deserialize`s from its RON variant name and
/// [resolves to](BoundKey::key_code) the matching Bevy [`KeyCode`]. The set is the
/// small keyboard subset the S8 acts need; extend it as later slices bind more
/// keys. Authored bare in RON via the variant name (`select_clear: KeyEscape`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum BoundKey {
    /// The `Escape` key.
    KeyEscape,
    /// The `Q` key.
    KeyQ,
    /// The `E` key.
    KeyE,
    /// The `C` key.
    KeyC,
    /// The `F` key.
    KeyF,
    /// The `R` key.
    KeyR,
    /// The `Tab` key.
    KeyTab,
    /// The `PageUp` key.
    KeyPageUp,
    /// The `PageDown` key.
    KeyPageDown,
    /// The bracket-left `[` key.
    KeyBracketLeft,
    /// The bracket-right `]` key.
    KeyBracketRight,
}

impl BoundKey {
    /// The Bevy [`KeyCode`] this authored key resolves to.
    ///
    /// The single typed [`BoundKey`]→[`KeyCode`] translation — a `const` mapping,
    /// not a literal scattered through the systems. Every keyboard system reads the
    /// resolved [`KeyCode`] off [`Keybinds`]; this is the only place a [`KeyCode`]
    /// is named.
    #[must_use]
    pub const fn key_code(self) -> KeyCode {
        match self {
            Self::KeyEscape => KeyCode::Escape,
            Self::KeyQ => KeyCode::KeyQ,
            Self::KeyE => KeyCode::KeyE,
            Self::KeyC => KeyCode::KeyC,
            Self::KeyF => KeyCode::KeyF,
            Self::KeyR => KeyCode::KeyR,
            Self::KeyTab => KeyCode::Tab,
            Self::KeyPageUp => KeyCode::PageUp,
            Self::KeyPageDown => KeyCode::PageDown,
            Self::KeyBracketLeft => KeyCode::BracketLeft,
            Self::KeyBracketRight => KeyCode::BracketRight,
        }
    }
}

/// The DATA-DRIVEN keybind table — every bound act → the [`BoundKey`] it is on.
///
/// Loaded from the loose `assets/input/keybinds.ron` through the generic GTW-136
/// [`RonAsset<T>`](gdtf_assets::RonAsset) loader and resolved into a presenter-side
/// resident [`Keybinds`] resource before battle time (see [`resolve_keybinds`]).
/// Every binding is data the engineer edits — nothing about the key choices is
/// hardcoded in Rust; this struct only names the bound ACTS. Each act resolves to
/// a [`KeyCode`] via [`BoundKey::key_code`], the no-hardcoded-literal accessor the
/// keyboard systems call.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored
/// `.ron` shape), and [`TypePath`] (the bound [`RonAsset<Keybinds>`] requires of
/// its payload).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Deserialize, TypePath)]
pub struct Keybinds {
    /// Clear the current [`SelectedShooter`](crate::SelectedShooter) selection.
    pub select_clear: BoundKey,
    /// Raise the presenter's [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel) by one storey.
    pub level_up:     BoundKey,
    /// Lower the presenter's [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel) by one storey.
    pub level_down:   BoundKey,
    /// Step the selected ganger's stance through the cyclic order (consumed in 222b).
    pub stance_cycle: BoundKey,
    /// Toggle the selected ganger's aim mode (consumed in 222b).
    pub aim_toggle:   BoundKey,
    /// Step the selected ganger's facing through the cyclic order (consumed in 222b).
    pub facing_cycle: BoundKey,
}

impl Keybinds {
    /// The [`KeyCode`] bound to clear-selection.
    #[must_use]
    pub const fn select_clear(&self) -> KeyCode {
        self.select_clear.key_code()
    }

    /// The [`KeyCode`] bound to level-up.
    #[must_use]
    pub const fn level_up(&self) -> KeyCode {
        self.level_up.key_code()
    }

    /// The [`KeyCode`] bound to level-down.
    #[must_use]
    pub const fn level_down(&self) -> KeyCode {
        self.level_down.key_code()
    }

    /// The [`KeyCode`] bound to stance-cycle (consumed in 222b).
    #[must_use]
    pub const fn stance_cycle(&self) -> KeyCode {
        self.stance_cycle.key_code()
    }

    /// The [`KeyCode`] bound to aim-toggle (consumed in 222b).
    #[must_use]
    pub const fn aim_toggle(&self) -> KeyCode {
        self.aim_toggle.key_code()
    }

    /// The [`KeyCode`] bound to facing-cycle (consumed in 222b).
    #[must_use]
    pub const fn facing_cycle(&self) -> KeyCode {
        self.facing_cycle.key_code()
    }
}

/// The in-flight handle to the keybind RON, held until it resolves into [`Keybinds`].
///
/// A named newtype over the bevy [`Handle`] (no-bare-types: a bare handle carries
/// no domain meaning; this name says "the keybind table being loaded"). Inserted by
/// [`load_keybinds`] and read by [`resolve_keybinds`] — the `TileRolesHandle`
/// precedent.
#[derive(Resource, Deref, Debug, Clone)]
pub struct KeybindsHandle(pub Handle<RonAsset<Keybinds>>);

/// `Startup`: kick off the `keybinds.ron` load, storing its typed handle.
///
/// Loads `input/keybinds.ron` as a [`RonAsset<Keybinds>`](gdtf_assets::RonAsset)
/// through the generic GTW-136 loader and inserts the [`KeybindsHandle`] the
/// [`resolve_keybinds`] poll system reads. Takes `Option<Res<AssetServer>>` so a
/// `MinimalPlugins` headless app with no [`AssetServer`] no-ops rather than
/// panicking (`bevy-traps.md` #1); under `DefaultPlugins` (the app + the
/// `AssetServer` harness) the load fires for real and the resolve runs.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the handle insert, the
/// optional [`Res<AssetServer>`] for the load.
pub fn load_keybinds(mut commands: Commands, asset_server: Option<Res<AssetServer>>) {
    let Some(asset_server) = asset_server else {
        return;
    };
    let handle = asset_server.load::<RonAsset<Keybinds>>(KEYBINDS_RON_PATH);
    commands.insert_resource(KeybindsHandle(handle));
}

/// `Update` (gated until [`Keybinds`] is resolved): resolve the loaded RON into the
/// resident [`Keybinds`] resource.
///
/// Once the [`RonAsset<Keybinds>`](gdtf_assets::RonAsset) has settled into
/// `Assets<RonAsset<Keybinds>>` (a transient one-frame "loaded but not yet in the
/// collection" state simply leaves it un-inserted this pass — retried next frame),
/// it copies the deserialized [`Keybinds`] out and inserts it as the resident
/// resource so the keyboard systems read it. Run only while [`KeybindsHandle`]
/// exists AND [`Keybinds`] does NOT (the plugin's run-condition), so it inserts
/// once — the `resolve_tile_roles` precedent.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the insert,
/// [`Res<KeybindsHandle>`] for the handle, [`Res<Assets<RonAsset<Keybinds>>>`] for
/// the loaded asset.
pub fn resolve_keybinds(
    mut commands: Commands,
    handle: Res<KeybindsHandle>,
    keybind_assets: Res<Assets<RonAsset<Keybinds>>>,
) {
    let Some(loaded) = keybind_assets.get(&**handle) else {
        // Loaded-but-not-yet-in-collection (or still loading) — retry next frame;
        // the run-condition keeps this system alive until Keybinds is resolved.
        return;
    };
    commands.insert_resource(**loaded);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC7 — the shipped keybind RON deserializes through the generic
    /// `ron::from_str` path the loader uses, and every declared act name resolves
    /// to a `KeyCode`. Parsing the embedded file contents proves the schema +
    /// the file agree (a malformed or incomplete file would fail here): serde
    /// rejects a missing field, so a successful parse proves all six bound acts
    /// are present and each resolves through [`BoundKey::key_code`].
    ///
    /// It does NOT pin the six authored `KeyCode` MAGNITUDES — `keybinds.ron` is
    /// editable, hot-swappable tuning data ("edit freely — every binding is data"),
    /// so locking the file's chosen keys would be a brittle test on editable data
    /// (the metric-constant exemption does not apply to keybinds). The non-brittle
    /// INVARIANT we do assert is that the six bound keys are mutually distinct —
    /// no two acts share a key, whatever the author binds them to. (The
    /// `fire_mode_cycle` binding was REMOVED in GTW-254 — the popup picker replaced
    /// the blind cycle.)
    #[test]
    fn shipped_keybinds_ron_deserializes_and_every_act_resolves() {
        // The exact bytes the loose `assets/input/keybinds.ron` ships, parsed the
        // same way `RonAssetLoader` parses them (`ron::de::from_bytes`).
        const RON: &str = include_str!("../../../assets/input/keybinds.ron");
        let parsed: Result<Keybinds, _> = ron::from_str(RON);
        assert!(
            parsed.is_ok(),
            "the shipped keybinds.ron must deserialize into Keybinds: {:?}",
            parsed.err(),
        );
        let Ok(binds) = parsed else { return };

        // Non-brittle invariant: the six bound keys are mutually distinct (no two
        // acts collide on the same key), independent of which keys the author chose.
        let bound = [
            binds.select_clear(),
            binds.level_up(),
            binds.level_down(),
            binds.stance_cycle(),
            binds.aim_toggle(),
            binds.facing_cycle(),
        ];
        for (i, lhs) in bound.iter().enumerate() {
            for rhs in &bound[i + 1..] {
                assert_ne!(
                    lhs, rhs,
                    "no two bound acts may share a key (shipped keybinds.ron has a collision)",
                );
            }
        }
    }

    /// Each `BoundKey` variant resolves to its documented `KeyCode` — the single
    /// typed translation the systems rely on (no hardcoded literal elsewhere).
    #[test]
    fn bound_key_resolves_to_its_key_code() {
        assert_eq!(BoundKey::KeyEscape.key_code(), KeyCode::Escape);
        assert_eq!(BoundKey::KeyPageUp.key_code(), KeyCode::PageUp);
        assert_eq!(BoundKey::KeyPageDown.key_code(), KeyCode::PageDown);
        assert_eq!(BoundKey::KeyBracketLeft.key_code(), KeyCode::BracketLeft);
        assert_eq!(BoundKey::KeyBracketRight.key_code(), KeyCode::BracketRight);
        assert_eq!(BoundKey::KeyTab.key_code(), KeyCode::Tab);
    }
}
