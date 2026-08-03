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

pub(super) fn ganger_scene(
    placed: &PlacedGanger,
    member: &GangMember,
    tuning: &GangerStatTuning,
) -> impl Scene {
    let at = placed.at;
    let name = (*member.name).clone();
    let faction = *placed.faction;
    let facing = *placed.facing;
    let stance = *placed.stance;
    let aiming = *placed.aiming;
    let attributes = member.attributes();
    let derived = derive_stats(&attributes, tuning);
    let speed = *member.speed;
    let aim = *member.aim;
    let strength = *member.strength;
    let toughness = *member.toughness;
    let reflexes = *member.reflexes;
    let cool = *member.cool;
    let grit = *member.grit;
    let luck = *member.luck;
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
    let life_state = placed.life_state;
    (
        bsn! {
            Position::new(at)
            GangerName::new(name)
            Faction::new(faction)
            Facing::new(facing)
            Stance::new(stance)
            Aiming::new(aiming)
            Speed::new(speed)
            Aim::new(aim)
            Strength::new(strength)
            Toughness::new(toughness)
            Reflexes::new(reflexes)
            Cool::new(cool)
            Grit::new(grit)
            Luck::new(luck)
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
            InflictedInjuries::default()
            BleedAfflicted::default()
            PeekOffset::default()
            ReactionsUsed::default()
        },
        template_value(life_state),
    )
}
