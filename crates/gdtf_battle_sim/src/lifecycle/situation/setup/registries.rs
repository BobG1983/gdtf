//! [`BattleSetup`] + [`BattleRegistries`] — the setup's public API surface: the
//! result type and the registry borrow-bundle
//! [`setup_battle`](super::setup_battle) resolves a
//! [`Situation`](crate::situation::Situation)'s authored references against.

use crate::{
    armor::ArmorRegistry,
    effects::fields::FieldDefRegistry,
    ganger::GangRegistry,
    occupancy::OccupantPlacement,
    terrain::def::TerrainDefRegistry,
    tuning::GangerStatTuning,
    weapon::{AttachmentRegistry, MeleeWeaponRegistry, WeaponRegistry},
};

/// The result of [`setup_battle`](super::setup_battle) — the spawned ganger placements, so the caller
/// can map each authored ganger to its newly-spawned Bevy [`Entity`](bevy::prelude::Entity) handle.
///
/// A named newtype over the placement list (no-bare-types: the setup outcome is a
/// domain value, not a bare `Vec`). Each [`OccupantPlacement`] pairs a
/// `(cell, level)` with the SPAWNED [`Entity`](bevy::prelude::Entity) handle — **never a numeric id**
/// (GTW-10 / GTW-12). The placements are in authored-ganger order. The seeded
/// [`CoverLedger`](crate::cover::CoverLedger) / [`SurfaceGrid`](crate::surface::SurfaceGrid) / [`OccupancyGrid`](crate::occupancy::OccupancyGrid) / [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph)
/// are inserted as resources, queried off the world rather than returned here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BattleSetup {
    /// The spawned `(cell, level) → Entity` occupant placements, in authored order.
    pub occupants: Vec<OccupantPlacement>,
}

impl BattleSetup {
    /// The number of gangers spawned by the setup.
    #[must_use]
    pub const fn ganger_count(&self) -> usize {
        self.occupants.len()
    }
}

/// The read-only content registries [`setup_battle`](super::setup_battle) resolves a
/// [`Situation`](crate::situation::Situation)'s authored references against — grouped into
/// one borrow-bundle so the setup's parameter list stays under clippy's argument-count gate
/// (the GTW-414 gang registry was the 8th argument; bundling the resolution sources keeps
/// the signature small).
///
/// A named borrow-bundle (no-bare-types: the setup's resolution sources are a domain
/// grouping, not a bare tuple of refs), the `gdtf_app` `LoadAssetCollections`-style
/// transparent-bundle precedent — it wraps existing world-state by reference, never a
/// domain scalar. Each field is the registry / tuning a setup phase looks an authored key
/// up in:
///
/// - `gangs` — resolve each [`PlacedGanger`](crate::situation::PlacedGanger)'s `(gang, member)` ref to a
///   [`GangMember`](crate::ganger::GangMember) (GTW-414).
/// - `weapons` / `armor` — resolve the resolved member's weapon / armor keys.
/// - `stat_tuning` — derive each ganger's computed stats from its eight attributes.
/// - `terrain` — resolve cover / slab terrain definition UUIDs against the
///   [`TerrainDefRegistry`] (GTW-491); `None` ⇒ cover / slab keys fail with `TerrainNotFound`
///   and the floor falls back to `fallback_floor_cost`.
#[derive(Clone, Copy)]
pub struct BattleRegistries<'a> {
    /// The gang rosters each placed ganger's `(gang, member)` ref resolves against (GTW-414).
    pub gangs:         &'a GangRegistry,
    /// The (ranged) weapon registry each resolved roster member's weapon key resolves against.
    pub weapons:       &'a WeaponRegistry,
    /// The MELEE weapon registry each resolved roster member's melee weapon key resolves
    /// against (GTW-505) — an authored key, or the [`fists`](crate::weapon::FISTS_KEY)
    /// default when the member authored none, so every ganger gets a melee weapon.
    pub melee_weapons: &'a MeleeWeaponRegistry,
    /// The armor registry each resolved roster member's armor key resolves against.
    pub armor:         &'a ArmorRegistry,
    /// The stat tuning each ganger's computed stats are derived with (GTW-384).
    pub stat_tuning:   &'a GangerStatTuning,
    /// The UUID-keyed terrain-definition registry cover / slab piece UUIDs resolve against
    /// (GTW-491); `None` skips terrain resolution (cover / slab keys then fail with
    /// `TerrainNotFound`, and the floor uses the [`fallback_floor_cost`](super::setup_battle)).
    pub terrain:       Option<&'a TerrainDefRegistry>,
    /// The area-damage-field catalog a situation's authored
    /// [`fields`](crate::situation::Situation::fields) placements resolve their
    /// [`FieldKey`](crate::effects::fields::FieldKey) against (GTW-545); `None` skips field seeding — a
    /// situation with an authored field then fails with
    /// [`FieldNotFound`](crate::situation::BattleSetupError::FieldNotFound), and a situation with NO fields
    /// (every test fixture that omits the list) seeds an empty
    /// [`FieldRegistry`](crate::effects::fields::FieldRegistry). The real app always has the catalog
    /// loaded before a battle starts.
    pub fields:        Option<&'a FieldDefRegistry>,
    /// The GTW-549 DATA-DRIVEN attachment registry — the key→spec map a weapon's
    /// [`attachments`](crate::weapon::WeaponSpec::attachments) keys resolve against; each
    /// resolved item's [`AttachmentEffect`](crate::weapon::AttachmentEffect)s ride onto the
    /// spawned weapon as its [`PendingAttachments`](crate::weapon::PendingAttachments) marker and are applied post-spawn by
    /// [`apply_pending_attachments`](crate::apply_pending_attachments). `None`
    /// ([`new`](Self::new)) skips attachment resolution — the fail-safe every content
    /// registry ref shares (a missing registry applies nothing, never fails a battle). The
    /// real app path passes the loaded value via [`with_attachments`](Self::with_attachments).
    /// SUPERSEDES the GTW-542 global-tuning attachment model (removed).
    pub attachments:   Option<&'a AttachmentRegistry>,
}

impl<'a> BattleRegistries<'a> {
    /// Build the resolution borrow-bundle from its six registry / tuning refs — the shape
    /// every [`setup_battle`](super::setup_battle) caller assembles (GTW-505 added `melee_weapons`).
    ///
    /// The GTW-545 field catalog + the GTW-549 attachment registry default to `None`, so
    /// existing callers (test fixtures without authored fields / attachments) skip those
    /// resolution phases; the app path attaches each via
    /// [`with_field_defs`](Self::with_field_defs) / [`with_attachments`](Self::with_attachments).
    #[must_use]
    pub const fn new(
        gangs: &'a GangRegistry,
        weapons: &'a WeaponRegistry,
        melee_weapons: &'a MeleeWeaponRegistry,
        armor: &'a ArmorRegistry,
        stat_tuning: &'a GangerStatTuning,
        terrain: Option<&'a TerrainDefRegistry>,
    ) -> Self {
        Self {
            gangs,
            weapons,
            melee_weapons,
            armor,
            stat_tuning,
            terrain,
            fields: None,
            attachments: None,
        }
    }

    /// The same borrow-bundle carrying the GTW-545 area-damage-field catalog — the app path
    /// passes the loaded [`FieldDefRegistry`] so a situation's authored
    /// [`fields`](crate::situation::Situation::fields) placements resolve their
    /// [`FieldKey`](crate::effects::fields::FieldKey) against the catalog. Defaults to `None`
    /// ([`new`](Self::new)), so existing callers (every test fixture without authored fields)
    /// seed an empty [`FieldRegistry`](crate::effects::fields::FieldRegistry) and never fail on the field-seed phase.
    #[must_use]
    pub const fn with_field_defs(mut self, fields: &'a FieldDefRegistry) -> Self {
        self.fields = Some(fields);
        self
    }

    /// The same borrow-bundle carrying the GTW-549 DATA-DRIVEN
    /// [`AttachmentRegistry`](crate::weapon::AttachmentRegistry) — the app path passes the
    /// loaded registry so a weapon's authored `attachments` keys resolve to their items and
    /// each item's effects ride onto the spawned weapon (applied post-spawn). Defaults to
    /// `None` ([`new`](Self::new)), so existing callers (every test fixture without authored
    /// attachments) skip attachment resolution and never fail.
    #[must_use]
    pub const fn with_attachments(mut self, attachments: &'a AttachmentRegistry) -> Self {
        self.attachments = Some(attachments);
        self
    }
}
