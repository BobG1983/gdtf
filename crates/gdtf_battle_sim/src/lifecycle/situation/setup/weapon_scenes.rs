//! [`wielded_weapon_scenes`] + [`wielded_melee_weapon_scenes`] — `bsn!` composition
//! of the wielded ranged + melee weapon entities (GTW-323 slice 2 / GTW-505 /
//! GTW-544 / GTW-547 / GTW-549 / GTW-554), spawn-and-related via
//! [`Wields`](crate::weapon::Wields) by [`setup_battle`](super::setup_battle).

use bevy::scene::{Scene, SceneList, bsn, bsn_list, template_value};

use crate::weapon::{
    Accuracy, BaseSpread, FatalBias, FightMode, FireMode, Kickback, MeleeWeapon, MeleeWeaponBundle,
    PendingAttachments, Reach, Shove, Stable, Weapon, WeaponBundle, WeaponDamage, WeaponName,
    WeaponPunch, WeaponShred, WeaponSpawnSiblings,
};

/// Compose the wielded-weapon **related scene list** for a ganger as a [`SceneList`] —
/// the single weapon entity carrying the full GTW-200 decomposed weapon-stat component
/// set, read by value from the resolved [`WeaponBundle`], to spawn-and-relate via
/// [`Wields`](crate::weapon::Wields) (GTW-323 slice 2, ADR-0004).
///
/// The [`setup_battle`](super::setup_battle) spawn loop hands this list to
/// [`queue_spawn_related_scenes::<Wields>`](bevy::scene::EntityCommandsSceneExt::queue_spawn_related_scenes)
/// on the freshly-spawned ganger entity: the framework spawns the weapon entity, applies
/// the scene's components, and inserts [`WieldedBy`](crate::weapon::WieldedBy)`(ganger)`
/// on it — whose back-reference hook populates the ganger's [`Wields`](crate::weapon::Wields) collection
/// automatically. So `fire()` reads the weapon stats off the **weapon entity** through
/// `ganger → Wields → the weapon entity`, mirroring the slice-1 armor traversal. The
/// list holds ONE entry this slice (a ganger wields a single weapon); the
/// [`queue_spawn_related_scenes`](bevy::scene::EntityCommandsSceneExt::queue_spawn_related_scenes)
/// surface takes a [`SceneList`], so the single weapon scene is wrapped in a one-element
/// `bsn_list!` (the same shape the multi-weapon loadout the ADR anticipates would take).
///
/// GTW-544/547: `siblings` are the weapon's optional `dot` / `on_death` sibling components
/// ([`WeaponSpawnSiblings`]). GTW-549: `pending` carries the resolved
/// [`AttachmentEffect`](crate::weapon::AttachmentEffect)s of the weapon's fitted attachment
/// items, composed onto the weapon entity as a [`PendingAttachments`] marker the post-spawn
/// [`apply_pending_attachments`](crate::apply_pending_attachments) system applies
/// via the [`attach_to_weapon`](crate::weapon::AttachToWeaponExt::attach_to_weapon) extension.
pub(super) fn wielded_weapon_scenes(
    weapon: &WeaponBundle,
    siblings: WeaponSpawnSiblings,
    pending: PendingAttachments,
) -> impl SceneList {
    bsn_list! { wielded_weapon_scene(weapon, siblings, pending) }
}

/// Compose ONE wielded-weapon entity as a `bsn!` [`Scene`] — the [`Weapon`] marker plus
/// the full GTW-200 decomposed weapon-stat component set, read by value from the
/// resolved [`WeaponBundle`] (GTW-323 slice 2, ADR-0004).
///
/// This weapon entity is the ONLY weapon storage — GTW-323 slice 3 removed the
/// transient on-ganger weapon copy, so no weapon stat data is stored on the ganger.
/// The [`Weapon`] marker, every weapon-number newtype, the [`WeaponName`], the
/// [`FireMode`], and the [`Stable`] tag inline via their `Type::new(value)` `bsn!`
/// form; the runtime-valued [`DamageType`](crate::weapon::DamageType) and the
/// value-typed [`Magazine`](crate::magazine::Magazine) — neither has a `bsn!` grammar
/// form — bridge via [`template_value`] and tuple-compose onto the same weapon entity.
/// The [`WieldedBy`](crate::weapon::WieldedBy) back-reference is inserted by the
/// framework's `queue_spawn_related_scenes::<Wields>` wiring, NOT here, so it is absent
/// from this scene.
///
/// GTW-544/547: the resolved `siblings` add the OPTIONAL `dot` / `on_death` sibling
/// components, each composed as an `Option<`[`template_value`]`>` (a `None` resolves to a
/// no-op, per `bevy_scene`'s `impl Scene for Option<S>`). GTW-549: `pending` is composed as a
/// [`PendingAttachments`] component (an EMPTY marker for a weapon with no attachments — the
/// application system then no-ops), which the post-spawn
/// [`apply_pending_attachments`](crate::apply_pending_attachments) system reads
/// to apply each attachment effect via the
/// [`attach_to_weapon`](crate::weapon::AttachToWeaponExt::attach_to_weapon) extension — the
/// mandated post-spawn `EntityCommand` path (the weapon entity's stat components exist once the
/// scene materializes).
fn wielded_weapon_scene(
    weapon: &WeaponBundle,
    siblings: WeaponSpawnSiblings,
    pending: PendingAttachments,
) -> impl Scene {
    // bsn! `Type::new(expr)` stores a DEFERRED constructor, so every captured value must
    // be OWNED (the GTW-322 `'static` finding). Read each stat by value out of the
    // bundle FIRST, then let the macro capture the owned locals (never the `&` param).
    let weapon_name = (*weapon.name).clone();
    let base_spread = *weapon.base_spread;
    let accuracy = *weapon.accuracy;
    let kickback = *weapon.kickback;
    let fatal_bias = *weapon.fatal_bias;
    let weapon_damage = *weapon.damage;
    let weapon_punch = *weapon.punch;
    let weapon_shred = *weapon.shred;
    let fire_mode = (*weapon.fire_mode).clone();
    let stable = *weapon.stable;
    // GTW-525: the `shove` knockback tag — WITHOUT seeding it the GTW-525 `WeaponQuery`-sibling
    // `&Shove` read (the fire-connect auto-shove hook's `shove_tags`) would not match the
    // spawned weapon entity, so a `shove`-tagged gun would never knock back in live play.
    let shove = *weapon.shove;
    // The runtime-valued / value-typed leaves with no `bsn!` grammar form, owned for the
    // `template_value` tuple-composition tail (the GTW-322 runtime-value path). The GTW-443
    // `Handedness` is a runtime-valued enum (like `DamageType`), so it bridges the same way
    // — WITHOUT it the GTW-443 `WeaponQuery`'s `&Handedness` column would not match the
    // spawned weapon entity and `fire()` would fail closed (an empty volley).
    let damage_type = weapon.damage_type;
    let magazine = weapon.magazine;
    let handedness = weapon.handedness;
    // GTW-546: the TrajectoryStyle — a runtime-valued enum (like `DamageType` / `Handedness`),
    // so it bridges via `template_value`. WITHOUT it the throw dispatch's `&TrajectoryStyle`
    // read would not match the spawned weapon entity, so a grenade's `trajectory: Arc` would
    // never register and the throw would fail closed (no lob).
    let trajectory = weapon.trajectory;
    // GTW-544: the OPTIONAL DotProfile sibling — a DOT weapon spawns its `{ damage,
    // DamageType, turns }` profile as a sibling component the fire path reads to attach a
    // `Dot` on a penetrating hit. A `None` (a non-DOT weapon) resolves to an Option-scene
    // no-op, so a non-DOT weapon spawns byte-identical. `DotProfile` derives Clone + Default
    // (from_profile is never called here — the PROFILE is spawned, not a live Dot) so
    // `template_value` applies (the sibling precedent).
    let dot = siblings.dot().map(template_value);
    // GTW-547: the OPTIONAL OnDeath sibling — a weapon authoring an `on_death` effect spawns its
    // OnDeath component so `resolve_on_death` fans it when the wielding ganger dies. A `None` (a
    // weapon with no death effect) resolves to an Option-scene no-op, so it spawns
    // byte-identical. `OnDeath` derives Clone + Default so `template_value` applies (the GTW-544
    // DotProfile sibling precedent); it is cloned out of the borrowed record (not `Copy`).
    let on_death = siblings.on_death().cloned().map(template_value);
    // GTW-549: the PendingAttachments marker — the resolved effects of the weapon's fitted
    // attachment items ride onto the weapon entity as this component (an EMPTY marker for a
    // weapon with no attachments), which the post-spawn `apply_pending_attachments` system reads
    // to apply each effect via `attach_to_weapon` (the mandated post-spawn EntityCommand — the
    // weapon entity's stat components exist once the scene materializes). `PendingAttachments`
    // derives Clone + Default so `template_value` applies (the sibling precedent); it is moved in
    // (not `Copy`).
    let pending = template_value(pending);
    (
        bsn! {
            Weapon
            WeaponName::new(weapon_name)
            BaseSpread::new(base_spread)
            Accuracy::new(accuracy)
            Kickback::new(kickback)
            FatalBias::new(fatal_bias)
            WeaponDamage::new(weapon_damage)
            WeaponPunch::new(weapon_punch)
            WeaponShred::new(weapon_shred)
            FireMode::new(fire_mode)
            Stable::new(stable)
            Shove::new(shove)
        },
        // The runtime-valued / value-typed components with no `bsn!` grammar form,
        // bridged via `template_value` and tuple-composed onto the SAME weapon entity.
        template_value(damage_type),
        template_value(magazine),
        template_value(handedness),
        // GTW-546: the per-weapon trajectory style (a grenade's `Arc` vs the default `Straight`).
        template_value(trajectory),
        // GTW-544: the optional DOT profile sibling (a `None` inserts nothing).
        dot,
        // GTW-547: the optional OnDeath effect sibling (a `None` inserts nothing).
        on_death,
        // GTW-549: the pending-attachments marker (empty for a weapon with no attachments — the
        // application system then no-ops), applied post-spawn via `attach_to_weapon`.
        pending,
    )
}

/// Compose the wielded MELEE-weapon **related scene list** for a ganger as a
/// [`SceneList`] — the single melee weapon entity carrying the GTW-505 decomposed
/// melee-weapon-stat component set, read by value from the resolved
/// [`MeleeWeaponBundle`], to spawn-and-relate via [`Wields`](crate::weapon::Wields) (GTW-505, the
/// [`wielded_weapon_scenes`] mirror).
///
/// The [`setup_battle`](super::setup_battle) spawn loop hands this list to
/// [`queue_spawn_related_scenes::<Wields>`](bevy::scene::EntityCommandsSceneExt::queue_spawn_related_scenes)
/// on the freshly-spawned ganger entity — exactly as the ranged weapon spawn does — so
/// the framework spawns the melee weapon entity, applies the scene's components, and
/// inserts [`WieldedBy`](crate::weapon::WieldedBy)`(ganger)` on it, populating the same
/// [`Wields`](crate::weapon::Wields) collection the ranged weapon is in. The melee entity carries the GTW-505
/// [`MeleeWeapon`] marker, so the ranged-firing path EXCLUDES it (GTW-505 C5). The list
/// holds ONE entry (a ganger wields a single melee weapon), wrapped in a one-element
/// `bsn_list!` like the ranged side. GTW-554: `pending` carries the melee weapon's
/// slot-gated resolved attachment effects (the ranged `pending` mirror).
pub(super) fn wielded_melee_weapon_scenes(
    weapon: &MeleeWeaponBundle,
    pending: PendingAttachments,
) -> impl SceneList {
    bsn_list! { wielded_melee_weapon_scene(weapon, pending) }
}

/// Compose ONE wielded MELEE-weapon entity as a `bsn!` [`Scene`] — the [`MeleeWeapon`]
/// marker plus the GTW-505 decomposed melee-weapon-stat component set, read by value
/// from the resolved [`MeleeWeaponBundle`] (the [`wielded_weapon_scene`] mirror).
///
/// The [`MeleeWeapon`] marker, the shared [`WeaponName`] / [`WeaponDamage`] /
/// [`WeaponPunch`] / [`WeaponShred`] / [`FatalBias`] newtypes, the melee [`Reach`], and
/// the [`FightMode`] selector inline via their `Type::new(value)` `bsn!` form; the
/// runtime-valued [`DamageType`](crate::weapon::DamageType) and [`Handedness`](crate::weapon::Handedness) (fieldless
/// enums with no `bsn!` grammar form) bridge via [`template_value`] and tuple-compose
/// onto the same melee weapon entity (the GTW-322 runtime-value path). The
/// [`WieldedBy`](crate::weapon::WieldedBy) back-reference is inserted by the framework's
/// `queue_spawn_related_scenes::<Wields>` wiring, NOT here.
///
/// GTW-554: `pending` — the melee weapon's slot-gated resolved attachment effects — is
/// composed as a [`PendingAttachments`] component via [`template_value`] (an EMPTY marker
/// for a melee weapon with no attachments; the post-spawn
/// [`apply_pending_attachments`](crate::apply_pending_attachments) system applies each
/// effect and removes the marker — the exact ranged-weapon path, so melee attachments are
/// FULLY supported).
fn wielded_melee_weapon_scene(
    weapon: &MeleeWeaponBundle,
    pending: PendingAttachments,
) -> impl Scene {
    // bsn! `Type::new(expr)` stores a DEFERRED constructor, so every captured value must
    // be OWNED (the GTW-322 `'static` finding). Read each stat by value out of the bundle
    // FIRST, then let the macro capture the owned locals (never the `&` param).
    let weapon_name = (*weapon.name).clone();
    let weapon_damage = *weapon.damage;
    let weapon_punch = *weapon.punch;
    let weapon_shred = *weapon.shred;
    let fatal_bias = *weapon.fatal_bias;
    let reach = *weapon.reach;
    let fight_mode = (*weapon.fight_mode).clone();
    // GTW-525: the `shove` knockback tag — WITHOUT seeding it the melee dispatch's
    // `MeleeWeaponQuery` `&Shove` column would not match the spawned melee weapon entity, so
    // `dispatch_melee` would fail closed (no strike) on EVERY ganger — the whole melee act would
    // silently die in live play (the GTW-443 `Handedness` seeding-or-fail-closed lesson).
    let shove = *weapon.shove;
    // The runtime-valued enums with no `bsn!` grammar form, owned for the
    // `template_value` tuple-composition tail (the GTW-322 runtime-value path).
    let damage_type = weapon.damage_type;
    let handedness = weapon.handedness;
    // GTW-554: the melee PendingAttachments marker (empty for a melee weapon with no
    // attachments — the application system then no-ops), applied post-spawn via
    // `attach_to_weapon` (the ranged-weapon `pending` precedent; Clone + Default so
    // `template_value` applies, moved in — not `Copy`).
    let pending = template_value(pending);
    (
        bsn! {
            MeleeWeapon
            WeaponName::new(weapon_name)
            WeaponDamage::new(weapon_damage)
            WeaponPunch::new(weapon_punch)
            WeaponShred::new(weapon_shred)
            FatalBias::new(fatal_bias)
            Reach::new(reach)
            FightMode::new(fight_mode)
            Shove::new(shove)
        },
        // The runtime-valued components with no `bsn!` grammar form, bridged via
        // `template_value` and tuple-composed onto the SAME melee weapon entity.
        template_value(damage_type),
        template_value(handedness),
        // GTW-554: the melee weapon's slot-gated pending attachment effects.
        pending,
    )
}
