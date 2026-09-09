//! The one GPU lock for this suite binary. Every test that builds a real wgpu device holds it,
//! so only one render app draws at a time however many threads libtest runs.

use std::sync::{Mutex, MutexGuard, PoisonError};

static GPU_LOCK: Mutex<()> = Mutex::new(());

// A failed test poisons the guard. The next test takes the lock anyway.
pub(crate) fn lock_gpu() -> MutexGuard<'static, ()> {
    GPU_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}
