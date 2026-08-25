//! `editor.paint` and the verdict-before-write ordering its reply depends on.

mod command;
#[cfg(test)]
mod test;

pub(in crate::net_qa) use command::EditorPaint;
