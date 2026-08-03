mod anchor;
mod classified;
mod classify;

#[cfg(test)]
mod test;

pub(in crate::actors::fx) use anchor::anchor_cell;
pub(in crate::actors::fx) use classified::ClassifiedPop;
pub(in crate::actors::fx) use classify::classify_report;
