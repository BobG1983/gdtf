//! Raw input a QA client drives the shell and the pointer with, one command per file.
pub(crate) mod activate;
pub(crate) mod click_cell;
pub(crate) mod focus_step;
pub(crate) mod hover;
pub(crate) mod keys;
pub(crate) mod press_key;
pub(crate) mod set_focus;
pub(crate) mod support;

#[cfg(test)]
mod test;

pub(crate) use activate::InputActivate;
pub(crate) use click_cell::InputClickCell;
pub(crate) use focus_step::InputFocusStep;
pub(crate) use hover::InputHover;
pub(crate) use press_key::InputPressKey;
pub(crate) use set_focus::InputSetFocus;
