//! Behavior-preserving split of the severity module tests (GTW-583) — one file
//! per concern, with the shared seeded-RNG + input-bundle fixtures in
//! [`support`].

mod support;

mod roll_term;
mod score_terms;
mod sourcing;
