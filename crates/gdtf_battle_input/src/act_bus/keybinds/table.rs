//! The authored key vocabulary, the resolved keybind table, and its RON loader.

use bevy::prelude::*;
use gdtf_assets::RonAsset;
use serde::Deserialize;

/// The path of the loose keybind RON, relative to the asset source root.
const KEYBINDS_RON_PATH: &str = "core_tuning/keybinds.tuning.ron";

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
/// Loaded from the loose `assets/core_tuning/keybinds.tuning.ron` through the generic GTW-136
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
    /// Cycle the [`SelectedShooter`](crate::SelectedShooter) to the NEXT player ganger
    /// (GTW-458). The `Tab` key: `cycle_selection_keys` reads this and pushes
    /// [`ActIntent::SelectNext`](crate::ActIntent::SelectNext) (or
    /// [`SelectPrev`](crate::ActIntent::SelectPrev) when `Shift` is held).
    pub select_next:  BoundKey,
    /// Cycle the [`SelectedShooter`](crate::SelectedShooter) to the PREVIOUS player ganger
    /// (GTW-458). The Prev half of the cycle chord — bound to the SAME key as
    /// [`select_next`](Self::select_next), differentiated by the held `Shift` modifier
    /// (`Tab` = Next, `Shift+Tab` = Prev). The on-bar Prev BUTTON pushes
    /// [`ActIntent::SelectPrev`](crate::ActIntent::SelectPrev) through the same seam.
    pub select_prev:  BoundKey,
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

    /// The [`KeyCode`] bound to select-next (the `Tab` cycle key — GTW-458).
    #[must_use]
    pub const fn select_next(&self) -> KeyCode {
        self.select_next.key_code()
    }

    /// The [`KeyCode`] bound to select-prev (the Prev half of the `Shift+Tab` cycle chord —
    /// GTW-458). Same key as [`select_next`](Self::select_next), differentiated by `Shift`.
    #[must_use]
    pub const fn select_prev(&self) -> KeyCode {
        self.select_prev.key_code()
    }
}

/// The in-flight handle to the keybind RON, held until it resolves into [`Keybinds`].
///
/// A named newtype over the bevy [`Handle`] (no-bare-types: a bare handle carries
/// no domain meaning; this name says "the keybind table being loaded"). Inserted by
/// [`load_keybinds`] and read by [`resolve_keybinds`] — the `TileRolesHandle`
/// precedent.
#[derive(Resource, Deref, Debug, Clone)]
pub struct KeybindsHandle(Handle<RonAsset<Keybinds>>);

impl KeybindsHandle {
    /// Wrap the in-flight `keybinds.tuning.ron` handle.
    #[must_use]
    pub const fn new(handle: Handle<RonAsset<Keybinds>>) -> Self {
        Self(handle)
    }
}

/// `Startup`: kick off the `keybinds.tuning.ron` load, storing its typed handle.
///
/// Loads `core_tuning/keybinds.tuning.ron` as a [`RonAsset<Keybinds>`](gdtf_assets::RonAsset)
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
    commands.insert_resource(KeybindsHandle::new(handle));
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
