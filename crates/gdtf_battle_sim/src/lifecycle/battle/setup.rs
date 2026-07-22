//! The battle-lifecycle SETUP driver: [`setup_battle_on_request`] — drain
//! [`SetupBattleRequested`], guard the `Load`-state registries, seed the RNG streams +
//! run [`setup_battle`], and signal [`BattleReady`] on `Ok` — E10.5 / GTW-212 /
//! GTW-257. The teardown driver lives in [`teardown`](super::teardown_battle_on_request).

use bevy::prelude::{Commands, MessageReader, MessageWriter, Res, error};

use super::runtime_seed::insert_battle_runtime;
use crate::{
    armor::ArmorRegistry,
    battle::messages::{BattleReady, SetupBattleRequested},
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    situation::{BattleRegistries, setup_battle},
    terrain::def::TerrainDefRegistry,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};

/// **Setup** the battle on [`SetupBattleRequested`] — seed the per-subsystem RNG
/// streams and run [`setup_battle`], signalling [`BattleReady`] on success (E10.5).
///
/// Drains [`MessageReader<SetupBattleRequested>`] and per message:
///
/// 1. Calls [`setup_battle`] on the REAL [`Commands`] path, resolving each ganger's
///    weapon key against the [`WeaponRegistry`] (GTW-257), each ganger's armor key
///    against the [`ArmorRegistry`] (GTW-269), and each cover/slab terrain definition
///    UUID against the [`TerrainDefRegistry`] (GTW-491). On `Ok` the sim resources
///    ([`CoverLedger`](crate::cover::CoverLedger) / [`SurfaceGrid`](crate::surface::SurfaceGrid) / [`OccupancyGrid`](crate::occupancy::OccupancyGrid) /
///    [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph) / [`FloorCostGrid`](crate::terrain::floor::FloorCostGrid)) and the spawned ganger entities
///    land in the world, the seven battle-lifetime per-subsystem RNG stream resources
///    ([`ShotRng`](crate::rng::ShotRng), [`SeverityRng`](crate::rng::SeverityRng), [`LootRng`](crate::rng::LootRng), [`InjuryRng`](crate::rng::InjuryRng), [`ProcgenRng`](crate::rng::ProcgenRng),
///    [`ReactionRng`](crate::rng::ReactionRng), [`FightRng`](crate::rng::FightRng)) are inserted (derived from the message's
///    [`BattleSeed`](crate::rng::BattleSeed) via the stable FNV-1a-64 label-hash,
///    GTW-14; each stream independent — a draw on one cannot perturb another; all
///    portable `ChaCha12Rng`-backed, byte-stable across builds and platforms for
///    cross-run replay), the [`BattleInProgress`](crate::battle::BattleInProgress) witness is inserted (the battle-active
///    tag the [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) band
///    gates on), the [`PlayerFaction`](crate::battle::PlayerFaction) is inserted seeded from
///    [`Situation::player_faction`](crate::situation::Situation), the
///    [`BattleRoster`](crate::battle::BattleRoster) is captured from the situation's fielded gangers' factions, the
///    [`ActiveFaction`](crate::turn::ActiveFaction) turn-cycle resource is seeded to the same player faction (the
///    player acts first; GTW-309), an EMPTY [`ActLog`](crate::act_log::ActLog) act log is
///    inserted (GTW-727 — the sim's ordered record of everything that happens, per-battle
///    so sequence numbering and its transition-detection maps reset clean), an EMPTY
///    [`SquadVisibility`](crate::visibility::SquadVisibility) squad fog is inserted
///    (GTW-341), the [`OmniscientFog`](crate::visibility::OmniscientFog) AI move fog is
///    inserted (GTW-70 — every in-bounds cell visible+explored) — all sharing
///    [`BattleInProgress`](crate::battle::BattleInProgress)'s lifetime — and a
///    [`BattleReady`] is written; on `Err` the typed
///    [`BattleSetupError`](crate::situation::BattleSetupError) (an invalid vertical link,
///    an unresolved weapon/armor/terrain key, or a below-minimum floor cost) is surfaced
///    via [`error!`] and NEITHER the six RNG streams / [`BattleInProgress`](crate::battle::BattleInProgress) /
///    [`PlayerFaction`](crate::battle::PlayerFaction) / [`BattleRoster`](crate::battle::BattleRoster) / [`ActiveFaction`](crate::turn::ActiveFaction) NOR [`BattleReady`] is
///    written — the app never advances on a bad battle, the gate never opens, and a
///    failed setup leaves NO orphaned RNG stream resources. NO
///    `unwrap`/`expect`/`panic`.
///
/// The [`GangRegistry`] (GTW-414), [`WeaponRegistry`], [`ArmorRegistry`], and
/// [`TerrainDefRegistry`] (GTW-491) are each read as `Option<Res<_>>` (PERSISTENT `Load`
/// state); a setup requested before ANY loads fails closed (logged, no [`BattleReady`]). The
/// [`GangRegistry`] resolves each [`PlacedGanger`](crate::situation::PlacedGanger)'s
/// `(gang, member)` ref into the roster member `setup_battle` derives the ganger from.
///
/// [`CombatTuning`](crate::tuning::CombatTuning) is NOT inserted here: it is E10.4's
/// PERSISTENT `Load` resource, present throughout the battle for the acts to read. It
/// IS read here as `Option<Res<_>>` to supply the `fallback_floor_cost`
/// (`CombatTuning::move_costs.open`) used as the uniform floor cost (GTW-491 retires the
/// per-floor registry move-cost resolution; per-floor move cost derived from `default_floor`
/// is deferred to GTW-482).
#[expect(
    clippy::too_many_arguments,
    reason = "the params are the message reader + writer, the gang / weapon / MELEE-weapon \
              (GTW-505) / armor / terrain / area-damage-field (GTW-545) / attachment \
              (GTW-549) registries, and the stat + combat tuning — each a distinct Bevy \
              SystemParam (Option<Res<_>> for the Load-state registries); the injection model \
              cannot be refactored to fewer without a wrapper resource that changes the API \
              surface"
)]
pub fn setup_battle_on_request(
    mut requests: MessageReader<SetupBattleRequested>,
    mut ready: MessageWriter<BattleReady>,
    gangs: Option<Res<GangRegistry>>,
    weapons: Option<Res<WeaponRegistry>>,
    melee_weapons: Option<Res<MeleeWeaponRegistry>>,
    armor: Option<Res<ArmorRegistry>>,
    terrain: Option<Res<TerrainDefRegistry>>,
    stat_tuning: Option<Res<GangerStatTuning>>,
    combat_tuning: Option<Res<CombatTuning>>,
    field_defs: Option<Res<crate::effects::fields::FieldDefRegistry>>,
    attachments: Option<Res<AttachmentRegistry>>,
    mut commands: Commands,
) {
    for request in requests.read() {
        // The weapon registry is E10.4-style PERSISTENT `Load` state, present before
        // any battle in the real app. This system runs UNGATED (before the
        // BattleInProgress-gated Simulate band), so it takes `Option<Res<_>>` to stay
        // panic-free if a setup is somehow requested before the registry loaded
        // (bevy-traps #1): a missing registry fails closed — no setup, no BattleReady.
        // GTW-414: the gang registry is the same PERSISTENT `Load` state, read as
        // `Option<Res<_>>` so a setup somehow requested before the gangs folder loaded
        // fails closed — no setup, no BattleReady (bevy-traps #1, mirroring the weapon
        // registry guard). Without it the PlacedGanger gang/member refs cannot resolve.
        let Some(gangs) = gangs.as_deref() else {
            error!(
                "battle setup requested but no GangRegistry is loaded; no BattleReady will be \
                 signalled (the gangs folder must load before a battle starts)"
            );
            continue;
        };
        let Some(weapons) = weapons.as_deref() else {
            error!(
                "battle setup requested but no WeaponRegistry is loaded; no BattleReady will be \
                 signalled (the weapons folder must load before a battle starts)"
            );
            continue;
        };
        // GTW-505: the MELEE weapon registry is the same PERSISTENT `Load` state, read as
        // `Option<Res<_>>` so a setup somehow requested before the melee weapons folder
        // loaded fails closed — no setup, no BattleReady (bevy-traps #1, mirroring the
        // ranged weapon registry guard). Without it (and the shipped `fists` default) an
        // un-authored ganger's melee weapon cannot resolve.
        let Some(melee_weapons) = melee_weapons.as_deref() else {
            error!(
                "battle setup requested but no MeleeWeaponRegistry is loaded; no BattleReady will \
                 be signalled (the melee weapons folder must load before a battle starts)"
            );
            continue;
        };
        // The armor registry is the same PERSISTENT `Load` state (GTW-269), read as
        // `Option<Res<_>>` so a setup somehow requested before it loaded fails closed —
        // no setup, no BattleReady (bevy-traps #1, mirroring the weapon registry guard).
        let Some(armor) = armor.as_deref() else {
            error!(
                "battle setup requested but no ArmorRegistry is loaded; no BattleReady will be \
                 signalled (the armor folder must load before a battle starts)"
            );
            continue;
        };
        // GTW-384: the GangerStatTuning is PERSISTENT `Load` state like CombatTuning,
        // resolved with a const-default fallback (its loader inserts a default on a
        // failed/missing file), so a setup somehow requested before it loaded does NOT
        // fail closed — it derives from the const-default weights (combat must never be
        // BLOCKED by missing balance data, matching CombatTuning's defaulting). Bind the
        // const default to a local so the borrow outlives the setup call.
        let default_stat_tuning = GangerStatTuning::default();
        let stat_tuning = stat_tuning.as_deref().unwrap_or(&default_stat_tuning);

        // GTW-396: the fallback floor cost — used when the situation omits
        // `default_floor` or when no terrain-definition registry is available. Sourced from
        // `CombatTuning::move_costs.open` (4 by default) to preserve pre-GTW-396
        // behavior for un-migrated test fixtures. Combat must never be blocked by
        // missing balance data.
        let default_combat_tuning = CombatTuning::default();
        let fallback_floor_cost = combat_tuning
            .as_deref()
            .map_or(default_combat_tuning.move_costs.open, |ct| {
                ct.move_costs.open
            });

        // GTW-491: the TerrainDefRegistry is PERSISTENT `Load` state (GTW-487, the UUID-keyed
        // terrain model), read as `Option<Res<_>>`. When absent,
        // `setup_battle` is called with `terrain: None` which:
        //  (a) makes cover/slab UUIDs fail with TerrainNotFound (a real situation's
        //      cover/slabs reference def UUIDs that need the registry);
        //  (b) uses the fallback_floor_cost for the floor grid (correct for test fixtures
        //      that use SituationBuilder without authored terrain).
        // The real app always has the registry loaded before a battle starts. Tests that use
        // SituationBuilder without cover/slabs pass `None` implicitly — they never authored
        // terrain UUIDs.
        let terrain_ref = terrain.as_deref();

        // GTW-545: the FieldDefRegistry is PERSISTENT `Load` state (the area-damage-field
        // catalog), read as `Option<Res<_>>`. When absent, `setup_battle` is called with
        // `fields: None` — a situation authoring a `fields:` placement then fails closed with
        // FieldNotFound, and a situation with no fields seeds an empty FieldRegistry. The real
        // app always has the catalog loaded before a battle starts.
        let field_defs_ref = field_defs.as_deref();

        // GTW-549 PHASE 1: the AttachmentRegistry is PERSISTENT `Load` state (the data-driven
        // attachment catalog), read as `Option<Res<_>>`. When absent, `setup_battle` is called
        // with `attachments: None` — a weapon's authored `attachments` keys then resolve to
        // nothing (the fail-safe). When present, each resolved item's effects ride onto the
        // spawned weapon as a PendingAttachments marker applied post-spawn by
        // `apply_pending_attachments`. The real app always has the catalog loaded before a
        // battle starts.
        let attachments_ref = attachments.as_deref();

        // Pour the situation into the world via the authoritative setup. A bad
        //    vertical link, a missing weapon/armor/terrain key, or a below-minimum
        //    floor cost returns the typed error — log it (NEVER panic / unwrap) and
        //    write NO BattleReady, so the app's gate never fires (fail-closed). The
        //    GangerStatTuning derives each ganger's computed stats from its eight
        //    authored attributes (GTW-384).
        let mut battle_registries = BattleRegistries::new(
            gangs,
            weapons,
            melee_weapons,
            armor,
            stat_tuning,
            terrain_ref,
        );
        // GTW-545: attach the area-damage-field catalog when it is loaded (the app path).
        if let Some(field_defs) = field_defs_ref {
            battle_registries = battle_registries.with_field_defs(field_defs);
        }
        // GTW-549: attach the data-driven attachment registry when it is loaded (the app
        // path) so each weapon's authored `attachments` keys resolve to their items' effects.
        if let Some(attachments) = attachments_ref {
            battle_registries = battle_registries.with_attachments(attachments);
        }
        match setup_battle(
            &request.situation,
            battle_registries,
            fallback_floor_cost,
            &mut commands,
        ) {
            Ok(_setup) => {
                // The battle is live: seed the battle-lifetime runtime resource set
                // (gate witness + RNG streams + factions + AI pacing + fog — see
                // `insert_battle_runtime`, the auditable mirror of teardown's remove
                // set), then signal BattleReady. Both happen ONLY on Ok.
                insert_battle_runtime(&mut commands, request);
                ready.write(BattleReady);
            }
            Err(e) => {
                error!(
                    "battle setup failed: {e} (an invalid vertical link, an unresolved \
                     weapon/armor/terrain key, a floor cost below the A* admissibility \
                     floor, or two gangers stacked on one spawn cell); no BattleReady \
                     will be signalled"
                );
            }
        }
    }
}
