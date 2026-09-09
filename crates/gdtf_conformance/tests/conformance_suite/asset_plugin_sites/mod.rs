//! Guard: outside the game and editor hosts, every asset plugin turns the file watcher
//! off, so no test app walks the asset tree when it is built.

mod check;
mod scan;
mod tree;
