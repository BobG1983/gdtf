mod fade;
mod height;
mod markers;
mod slide;

crate::support_use!(markers::{CombatLogLine, CombatLogRoot};);

pub(crate) use fade::LogLineFade;
pub(crate) use height::PanelHeightAnim;
pub(crate) use slide::LineSlide;

#[cfg(test)]
mod test;
