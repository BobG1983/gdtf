//! Host lifecycle integration tests (launch, stop, output, orphans).

mod child_dir;
mod child_output;
mod fake_child;
mod instances;
mod orphan;
mod output_tail;
mod process;
mod production_wiring;
mod reaper;
mod recipe;
mod support;
mod sweep;
#[cfg(unix)]
mod system_watch;
mod tail_order;
