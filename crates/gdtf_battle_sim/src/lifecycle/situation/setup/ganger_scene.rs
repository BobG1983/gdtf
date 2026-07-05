//! [`ganger_scene`] — `bsn!` composition of the ganger's OWN component tree
//! (GTW-322 / GTW-384 / GTW-438), spawned once per placed ganger by
//! [`setup_battle`](super::setup_battle).

use bevy::scene::{Scene, bsn, template_value};

use crate::{
    ganger::{
        Aim, Aiming, Bottle, Cool, Facing, Faction, Fight, GangMember, GangerName, Grit, Hp, HpMax,
        Luck, Morale, Position, Reactions, Reflexes, Shooting, Speed, Stance, Strength, Toughness,
        Tu, TuMax, Wounds, WoundsMax, derive_stats,
    },
    inflicted_wound::InflictedWounds,
    injuries::{BleedAfflicted, InflictedInjuries},
    los::PeekOffset,
    situation::PlacedGanger,
    tuning::{GangerStatTuning, ReactionsUsed},
};

/// Compose ONE ganger as a Bevy `bsn!` [`Scene`] — the per-field component tree
/// [`setup_battle`](super::setup_battle) spawns for each [`GangerSpawn`](crate::situation::GangerSpawn)
/// (GTW-322).
///
/// The ganger carries its OWN state only — **no equipment stat data**. Since GTW-323
/// slice 3 (ADR-0004) the weapon and armor stats live exclusively on the related
/// weapon ([`Wields`](crate::weapon::Wields)) and armor-piece ([`Wears`](crate::armor::Wears)) entities spawned alongside the
/// ganger (see [`wielded_weapon_scenes`](super::weapon_scenes::wielded_weapon_scenes) / [`worn_piece_scenes`](super::armor_scenes::worn_piece_scenes)); the ganger holds
/// the relationship, never the components. So this scene composes the per-field ganger
/// state ([`Position`] / [`GangerName`] / [`Faction`] / [`Facing`] / [`Stance`] /
/// [`Aiming`] / [`LifeState`](crate::ganger::LifeState)), the EIGHT authored DIRECT
/// ATTRIBUTES ([`Speed`] / [`Aim`] / [`Strength`] / [`Toughness`] / [`Reflexes`] /
/// [`Cool`] / [`Grit`] / [`Luck`] — the raw potential), the DERIVED computed stats
/// ([`Shooting`] / [`Tu`] / [`TuMax`] / [`Hp`] / [`HpMax`] / [`Wounds`] / [`WoundsMax`]
/// plus the dormant [`Fight`] / [`Reactions`] / [`Morale`] / [`Bottle`]) computed at
/// setup from the attributes × the passed [`GangerStatTuning`] (GTW-384 — fully derived,
/// never authored: the situation authors attributes, the pools/skills are derived
/// here, full at battle start so the current pool == max), and the GTW-279 empty
/// [`InflictedWounds`] record — and nothing else.
///
/// **The `bsn!` recipe (GTW-322 spike).** Every newtype with a `Type::new(value)`
/// constructor is inlined in `bsn!`. The runtime-valued fieldless enum
/// [`LifeState`](crate::ganger::LifeState) (no `new(value)` whole-value ctor) has NO
/// `bsn!` grammar form — inlining a variant would NARROW the authored value — so it is
/// bridged via [`template_value`](bevy::scene::template_value) and tuple-composed onto
/// the SAME root entity. The composed type carries a GTW-322 spawn-seed-sentinel
/// [`Default`].
///
/// **Deferred materialization.** `bsn!`-scene components materialize on the
/// `SpawnScene` schedule (~one `app.update()` later), NOT synchronously. The caller
/// keys occupancy off the [`GangerSpawn`](crate::situation::GangerSpawn)'s authored
/// [`at`](crate::situation::GangerSpawn::at) value and the synchronously-reserved
/// `Entity` id (`spawn_scene(..).id()`), never off the deferred [`Position`]
/// component.
pub(super) fn ganger_scene(
    placed: &PlacedGanger,
    member: &GangMember,
    tuning: &GangerStatTuning,
) -> impl Scene {
    // The `bsn!` `Type::new(expr)` form stores a DEFERRED constructor, so every value
    // it captures must be OWNED/`'static` — a borrow (`&PlacedGanger` / `&GangMember`)
    // captured into the macro would make the returned scene outlive the reference (the
    // GTW-322 spike's `'static` finding). So bind every inline value to an owned local
    // FIRST, and let the macro capture those owned locals (never the `&` param).
    //
    // GTW-414 schema v2: the placement fields (`at` / `faction` / `facing` / `stance` /
    // `aiming` / `life_state`) come from the situation-side `PlacedGanger`; the identity
    // (`name`) and the eight direct attributes come from the gang-roster `GangMember`.
    let at = placed.at;
    let name = (*member.name).clone();
    let faction = *placed.faction;
    let facing = *placed.facing;
    let stance = *placed.stance;
    let aiming = *placed.aiming;
    // GTW-384: DERIVE every computed stat from the eight roster attributes × the
    // tuning (the single source of truth — `derive_stats`). The gang roster carries the
    // attributes only; the pools/skills are derived here, full at battle start
    // (current pool == max).
    let attributes = member.attributes();
    let derived = derive_stats(&attributes, tuning);
    // The eight roster attribute magnitudes (the raw potential, carried on the ganger).
    let speed = *member.speed;
    let aim = *member.aim;
    let strength = *member.strength;
    let toughness = *member.toughness;
    let reflexes = *member.reflexes;
    let cool = *member.cool;
    let grit = *member.grit;
    let luck = *member.luck;
    // The derived computed-stat magnitudes (Deref'd out of the DerivedStats record).
    let shooting = *derived.shooting;
    let fight = *derived.fight;
    let reactions = *derived.reactions;
    let morale = *derived.morale;
    let tu = *derived.tu;
    let tu_max = *derived.tu_max;
    let hp = *derived.hp;
    let hp_max = *derived.hp_max;
    let wounds = *derived.wounds;
    let wounds_max = *derived.wounds_max;
    let bottle = *derived.bottle;
    // The runtime-valued leaf with no `bsn!` grammar form, owned for the
    // `template_value` tuple-composition tail (see the doc-comment recipe).
    let life_state = placed.life_state;
    (
        bsn! {
            Position::new(at)
            GangerName::new(name)
            Faction::new(faction)
            Facing::new(facing)
            Stance::new(stance)
            Aiming::new(aiming)
            // The eight authored DIRECT ATTRIBUTES (the raw potential).
            Speed::new(speed)
            Aim::new(aim)
            Strength::new(strength)
            Toughness::new(toughness)
            Reflexes::new(reflexes)
            Cool::new(cool)
            Grit::new(grit)
            Luck::new(luck)
            // The DERIVED computed stats (attributes × GangerStatTuning, full at start).
            Shooting::new(shooting)
            Fight::new(fight)
            Reactions::new(reactions)
            Morale::new(morale)
            Tu::new(tu)
            TuMax::new(tu_max)
            Hp::new(hp)
            HpMax::new(hp_max)
            Wounds::new(wounds)
            WoundsMax::new(wounds_max)
            Bottle::new(bottle)
            InflictedWounds::default()
            // GTW-438: seed the EMPTY injury ledger so every ganger carries
            // InflictedInjuries from frame 0 — the `apply_injury` boundary needs it
            // present to `gain` into, and the GTW-436 projector / inspect read it. It
            // derives Default + Clone + Component (the bsn! sentinel-Default requirement,
            // bsn-sentinel-defaults convention), starting at the zero-delta empty ledger.
            InflictedInjuries::default()
            // GTW-438: seed the EMPTY injury-bleed accrual so the bleed runtime's
            // `Option<&BleedAfflicted>` query matches every ganger; `apply_injury` keeps it
            // in sync with the ledger's accrued bleed. Starts at the no-bleed `0`.
            BleedAfflicted::default()
            // GTW-406: seed the wall-peek offset so the automatic positional populator's
            // `&mut PeekOffset` query matches every ganger from frame 0 (a correctness
            // prerequisite — the populator skips a ganger that lacks the component). It
            // derives Default + Clone + Component (the bsn! sentinel-Default requirement),
            // and starts at the centred, no-peek `Vec2::ZERO`.
            PeekOffset::default()
            // GTW-468: seed the per-turn reaction-interrupt counter so the live reaction
            // trigger's `&mut ReactionsUsed` query (and the §8 `may_interrupt` cap gate)
            // matches every ganger from frame 0 — a ganger lacking it could never react and
            // could never be cap-gated. It derives Default + Component (the bsn!
            // sentinel-Default requirement), starting at `0` (a fresh turn, no interrupts
            // used). The turn-boundary `reset_reactions_used` zeroes it each turn (C6).
            ReactionsUsed::default()
        },
        // The runtime-valued component with no `bsn!` grammar form, bridged via
        // `template_value` and tuple-composed onto the SAME root entity (the GTW-322
        // spike's canonical runtime-value path).
        template_value(life_state),
    )
}
