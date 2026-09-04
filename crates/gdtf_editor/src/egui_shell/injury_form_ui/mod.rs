mod autoload;
mod def_panel;
mod fields;
mod panels;
mod weighting_panel;

pub(crate) use autoload::{autoload_first_injury, autoload_weighting_table};
pub(crate) use def_panel::def_panel;
pub(crate) use fields::field_stack;
pub(crate) use panels::{InjuryPanelsCtx, injury_panels};
pub(crate) use weighting_panel::weighting_panel;
