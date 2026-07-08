//! The **attachment authoring spec** — the [`AttachmentSpec`] an
//! `assets/content/attachments/*.attachment.ron` deserializes into (GTW-549, PHASE 1; GTW-558
//! re-homed into the attachment MECHANICS module). A data-driven RON item carrying a display
//! name + a typed list of [`AttachmentEffect`](crate::effects::attachments::AttachmentEffect)s,
//! each with its per-item magnitude.

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::AttachmentSlot;
use crate::{effects::attachments::AttachmentEffect, weapon::WeaponName};

/// The **authoring struct** an `assets/content/attachments/*.attachment.ron`
/// deserializes into (GTW-549) — the data-driven attachment item that SUPERSEDES the
/// GTW-542 closed `AttachTag` enum (removed). A weapon references it BY KEY (the
/// file stem) in its [`attachments`](crate::weapon::WeaponSpec::attachments).
///
/// It carries a human-facing `display_name` (the weapon/melee/armor spec precedent — a
/// picker / HUD label), the SINGLE [`AttachmentSlot`] the item occupies (`slot`, GTW-554 —
/// the fit gate admits it only into a weapon declaring that slot with free capacity),
/// plus `effects`, a typed
/// `Vec<`[`AttachmentEffect`](crate::effects::attachments::AttachmentEffect)`>` where EACH
/// effect carries its OWN magnitude/payload (the headline GTW-549 fix: magnitudes live on the
/// item, never in global tuning). An EMPTY `effects:` list is the identity (a cosmetic
/// attachment — applies nothing).
///
/// Derives [`Deserialize`] so the loose `.ron` parses (the closed
/// [`AttachmentEffect`](crate::effects::attachments::AttachmentEffect) enum is the serde
/// name↔type bridge), [`Serialize`] so the content editor's ATTACHMENT authoring mode
/// saves the SAME schema it loads (GTW-669 — the `GangRoster` / `ArmorSpec` round-trip
/// precedent; the sim never serializes at runtime), and [`TypePath`] because the
/// `RonAsset<AttachmentSpec>` the folder
/// loader wraps it in requires its payload to be [`TypePath`] (the
/// [`WeaponSpec`](crate::weapon::WeaponSpec) /
/// [`MeleeWeaponSpec`](crate::weapon::MeleeWeaponSpec) precedent).
///
/// **Not `Copy`** — it owns a `Vec` (and a [`WeaponName`] display name); it is `Clone`, so
/// the [`AttachmentRegistry`](super::AttachmentRegistry) can hold specs BY VALUE.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TypePath)]
pub struct AttachmentSpec {
    /// The attachment's human-facing name (`display_name`) — the picker / HUD label. A
    /// [`WeaponName`] (reusing the weapon-identity newtype for a display string, the
    /// no-bare-types rule); it is NOT the item KEY (the key is the file stem, supplied by
    /// the loader).
    pub display_name: WeaponName,
    /// The SINGLE [`AttachmentSlot`] this item occupies (`slot`, GTW-554) — authored as the
    /// bare variant name (`slot: Muzzle`). REQUIRED (no serde default): every item must
    /// declare its mount point, because the fit gate
    /// ([`attachment_fits`](super::attachment_fits)) admits an item only into a weapon that
    /// declares this slot with free capacity. There is deliberately NO ranged/melee class
    /// tag — class gating EMERGES from which slots a weapon offers.
    pub slot:         AttachmentSlot,
    /// The typed list of
    /// [`AttachmentEffect`](crate::effects::attachments::AttachmentEffect)s this item applies
    /// to its weapon (`effects`), each carrying its per-item magnitude. An EMPTY list is the
    /// identity. `#[serde(default)]` so a cosmetic attachment that authors no `effects:`
    /// field parses to an empty list.
    #[serde(default)]
    pub effects:      Vec<AttachmentEffect>,
}
