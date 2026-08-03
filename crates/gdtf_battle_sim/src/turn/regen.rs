use crate::{
    ganger::{Faction, Tu, TuMax},
    tu::reset_tu,
};

pub fn regen_team_tu<'a, I>(gangers: I, team: Faction)
where
    I: IntoIterator<Item = (&'a Faction, &'a mut Tu, &'a TuMax)>,
{
    for (faction, tu, tu_max) in gangers {
        if *faction == team {
            reset_tu(tu, tu_max);
        }
    }
}
