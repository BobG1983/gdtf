//! The authored key vocabulary, the resolved keybind table, and its hot-RON
//! chain registration (the GTW-564 generic hot-RON load path).

use bevy::prelude::*;
use gdtf_assets::HotRonAppExt;
use serde::Deserialize;

use crate::contextual::SlotRank;

/// The path of the loose keybind RON, relative to the asset source root.
const KEYBINDS_RON_PATH: &str = "core_tuning/keybinds.tuning.ron";

/// The fixed contextual-slot digit vocabulary (GTW-563): index `i` (0-based) holds the
/// [`BoundKey`] that activates visible slot rank `i + 1` — digit N activates the Nth
/// currently-visible contextual button (the user's "number maps to number" rule).
///
/// Nine entries — headroom over the eight registered contextual acts. Unlike the authored
/// ACT keys this mapping is NOT a tunable table field: the slot→digit correspondence is
/// fixed by the UX rule (a per-slot RON field would let an author break slot = number).
/// Each entry still resolves its concrete [`KeyCode`] through [`BoundKey::key_code`], so no
/// [`KeyCode`] literal is ever named at a call site.
const CONTEXTUAL_SLOT_KEYS: [BoundKey; 9] = [
    BoundKey::KeyDigit1,
    BoundKey::KeyDigit2,
    BoundKey::KeyDigit3,
    BoundKey::KeyDigit4,
    BoundKey::KeyDigit5,
    BoundKey::KeyDigit6,
    BoundKey::KeyDigit7,
    BoundKey::KeyDigit8,
    BoundKey::KeyDigit9,
];

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
    /// The `V` key.
    KeyV,
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
    /// The `1` digit key (contextual slot 1 — GTW-563).
    KeyDigit1,
    /// The `2` digit key (contextual slot 2 — GTW-563).
    KeyDigit2,
    /// The `3` digit key (contextual slot 3 — GTW-563).
    KeyDigit3,
    /// The `4` digit key (contextual slot 4 — GTW-563).
    KeyDigit4,
    /// The `5` digit key (contextual slot 5 — GTW-563).
    KeyDigit5,
    /// The `6` digit key (contextual slot 6 — GTW-563).
    KeyDigit6,
    /// The `7` digit key (contextual slot 7 — GTW-563).
    KeyDigit7,
    /// The `8` digit key (contextual slot 8 — GTW-563).
    KeyDigit8,
    /// The `9` digit key (contextual slot 9 — GTW-563).
    KeyDigit9,
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
            Self::KeyV => KeyCode::KeyV,
            Self::KeyTab => KeyCode::Tab,
            Self::KeyPageUp => KeyCode::PageUp,
            Self::KeyPageDown => KeyCode::PageDown,
            Self::KeyBracketLeft => KeyCode::BracketLeft,
            Self::KeyBracketRight => KeyCode::BracketRight,
            Self::KeyDigit1 => KeyCode::Digit1,
            Self::KeyDigit2 => KeyCode::Digit2,
            Self::KeyDigit3 => KeyCode::Digit3,
            Self::KeyDigit4 => KeyCode::Digit4,
            Self::KeyDigit5 => KeyCode::Digit5,
            Self::KeyDigit6 => KeyCode::Digit6,
            Self::KeyDigit7 => KeyCode::Digit7,
            Self::KeyDigit8 => KeyCode::Digit8,
            Self::KeyDigit9 => KeyCode::Digit9,
        }
    }
}

/// The DATA-DRIVEN keybind table — every bound act → the [`BoundKey`] it is on.
///
/// Loaded from the loose `assets/core_tuning/keybinds.tuning.ron` through the GTW-564
/// generic hot-RON chain (`register_keybinds_hot_ron`) and resolved into a
/// resident [`Keybinds`] resource before battle time.
/// Every binding is data the engineer edits — nothing about the key choices is
/// hardcoded in Rust; this struct only names the bound ACTS. Each act resolves to
/// a [`KeyCode`] via [`BoundKey::key_code`], the no-hardcoded-literal accessor the
/// keyboard systems call.
///
/// Derives [`Resource`] (the resolved runtime form), [`Deserialize`] (the authored
/// `.ron` shape), and [`TypePath`] (the bound [`RonAsset<Keybinds>`](gdtf_assets::RonAsset) requires of
/// its payload).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Deserialize, TypePath)]
pub struct Keybinds {
    /// Clear the current [`SelectedShooter`](crate::SelectedShooter) selection.
    pub select_clear:     BoundKey,
    /// Raise the presenter's [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel) by one storey.
    pub level_up:         BoundKey,
    /// Lower the presenter's [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel) by one storey.
    pub level_down:       BoundKey,
    /// Toggle the presenter's [`ViewMode`](gdtf_battle_presenter::ViewMode) between
    /// `DownToActive` (draw `0..=active`) and `FullView` (draw ALL storeys — the UFO
    /// full-stack view) (GTW-521). The keyboard [`full_view_key`](crate::keyboard::full_view_key)
    /// reads this and pushes [`ActIntent::ToggleFullView`](crate::ActIntent::ToggleFullView).
    pub toggle_full_view: BoundKey,
    /// Step the selected ganger's stance through the cyclic order (consumed in 222b).
    pub stance_cycle:     BoundKey,
    /// Toggle the selected ganger's aim mode (consumed in 222b).
    pub aim_toggle:       BoundKey,
    /// Step the selected ganger's facing through the cyclic order (consumed in 222b).
    pub facing_cycle:     BoundKey,
    /// Cycle the [`SelectedShooter`](crate::SelectedShooter) to the NEXT player ganger
    /// (GTW-458). The `Tab` key: `cycle_selection_keys` reads this and pushes
    /// [`ActIntent::SelectNext`](crate::ActIntent::SelectNext) (or
    /// [`SelectPrev`](crate::ActIntent::SelectPrev) when `Shift` is held).
    pub select_next:      BoundKey,
    /// Cycle the [`SelectedShooter`](crate::SelectedShooter) to the PREVIOUS player ganger
    /// (GTW-458). The Prev half of the cycle chord — bound to the SAME key as
    /// [`select_next`](Self::select_next), differentiated by the held `Shift` modifier
    /// (`Tab` = Next, `Shift+Tab` = Prev). The on-bar Prev BUTTON pushes
    /// [`ActIntent::SelectPrev`](crate::ActIntent::SelectPrev) through the same queue.
    pub select_prev:      BoundKey,
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

    /// The [`KeyCode`] bound to the full-view toggle (GTW-521).
    #[must_use]
    pub const fn toggle_full_view(&self) -> KeyCode {
        self.toggle_full_view.key_code()
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

    /// The digit [`KeyCode`] that activates contextual panel slot `rank` — the Nth
    /// currently-visible contextual button, 1-based — or [`None`] when `rank` exceeds the
    /// nine-digit vocabulary (GTW-563).
    ///
    /// The slot→digit mapping is FIXED (digit N activates slot N, the user's "number maps
    /// to number" UX rule), so unlike the authored ACT keys it is a constant translation,
    /// not a tunable table field — hence an ASSOCIATED function reading nothing off the
    /// resident table (`Keybinds::contextual_slot_key(rank)`), backed by a fixed nine-key
    /// digit vocabulary. It still resolves the concrete [`KeyCode`] through
    /// [`BoundKey::key_code`], so no [`KeyCode`] literal is named at the call site.
    #[must_use]
    pub fn contextual_slot_key(rank: SlotRank) -> Option<KeyCode> {
        // `rank` is 1-based; drop to the 0-based vocabulary index (rank 0 -> no key).
        let index = (*rank).checked_sub(1)?;
        CONTEXTUAL_SLOT_KEYS
            .get(usize::from(index))
            .map(|key| key.key_code())
    }
}

/// Registers the [`Keybinds`] hot-RON chain — ONE ext call onto the GTW-564
/// generic hot-RON load path (kick-off / gated resolve / live redrive, keyed by the generic
/// [`HotRonHandle`](gdtf_assets::HotRonHandle)`<Keybinds>`), replacing the
/// per-site handle newtype + load/resolve/redrive triple. Self-gates on the
/// [`AssetServer`](bevy::asset::AssetServer) (`bevy-traps.md` #1), so a
/// `MinimalPlugins` headless app stays a no-op ([`Keybinds`] is then absent and
/// every keyboard system stays gated on its presence).
///
/// On a live `core_tuning/keybinds.tuning.ron` edit the generic redrive
/// overwrites the resident [`Keybinds`] through `ResMut` ([`Keybinds`] IS both
/// the `Deserialize` payload AND the runtime `Resource` — no `resolve()` step),
/// so the keyboard systems read the new bindings the very next frame — WITHOUT
/// a rebuild or restart (the GTW-533 live keybind hot-reload, preserved).
pub(crate) fn register_keybinds_hot_ron(app: &mut App) {
    app.init_hot_ron_resource::<Keybinds>(KEYBINDS_RON_PATH);
}
