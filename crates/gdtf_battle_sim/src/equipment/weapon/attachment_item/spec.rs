//! The **attachment authoring spec** — the [`AttachmentSpec`] an
//! `assets/content/attachments/*.attachment.ron` deserializes into (GTW-549, PHASE 1). A
//! data-driven RON item carrying a display name + a typed list of
//! [`AttachmentEffect`](super::AttachmentEffect)s, each with its per-item magnitude.

use bevy::reflect::TypePath;
use serde::Deserialize;

use super::AttachmentEffect;
use crate::weapon::WeaponName;

/// The **authoring struct** an `assets/content/attachments/*.attachment.ron`
/// deserializes into (GTW-549) — the data-driven attachment item that SUPERSEDES the
/// GTW-542 closed `AttachTag` enum (removed). A weapon references it BY KEY (the
/// file stem) in its [`attachments`](crate::weapon::WeaponSpec::attachments).
///
/// It carries a human-facing `display_name` (the weapon/melee/armor spec precedent — a
/// picker / HUD label) + `effects`, a typed `Vec<`[`AttachmentEffect`](super::AttachmentEffect)`>`
/// where EACH effect carries its OWN magnitude/payload (the headline GTW-549 fix:
/// magnitudes live on the item, never in global tuning). An EMPTY `effects:` list is the
/// identity (a cosmetic attachment — applies nothing).
///
/// Derives [`Deserialize`] so the loose `.ron` parses (the closed
/// [`AttachmentEffect`](super::AttachmentEffect) enum is the serde name↔type bridge), and
/// [`TypePath`] because the `RonAsset<AttachmentSpec>` the folder loader wraps it in
/// requires its payload to be [`TypePath`] (the [`WeaponSpec`](crate::weapon::WeaponSpec) /
/// [`MeleeWeaponSpec`](crate::weapon::MeleeWeaponSpec) precedent).
///
/// **Not `Copy`** — it owns a `Vec` (and a [`WeaponName`] display name); it is `Clone`, so
/// the [`AttachmentRegistry`](super::AttachmentRegistry) can hold specs BY VALUE.
#[derive(Debug, Clone, PartialEq, Deserialize, TypePath)]
pub struct AttachmentSpec {
    /// The attachment's human-facing name (`display_name`) — the picker / HUD label. A
    /// [`WeaponName`] (reusing the weapon-identity newtype for a display string, the
    /// no-bare-types rule); it is NOT the item KEY (the key is the file stem, supplied by
    /// the loader).
    pub display_name: WeaponName,
    /// The typed list of [`AttachmentEffect`](super::AttachmentEffect)s this item applies to
    /// its weapon (`effects`), each carrying its per-item magnitude. An EMPTY list is the
    /// identity. `#[serde(default)]` so a cosmetic attachment that authors no `effects:`
    /// field parses to an empty list.
    #[serde(default)]
    pub effects:      Vec<AttachmentEffect>,
}
