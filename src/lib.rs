//! repo39 — repository exploration as a library.
//!
//! Core operations (`run_identify`, `run_map`, `run_deps`, `run_changes`,
//! `run_search`, `run_review`) are exposed so downstream crates (e.g.
//! agent39) can call them directly. Each writes its output to a
//! caller-supplied `&mut impl Write`.
//!
//! The CLI binary and the MCP server live in the bin target and are not part
//! of the library surface.

pub mod changes;
pub mod deps;
pub mod identify;
pub mod map;
pub mod read;
pub mod review;
pub mod search;

// Internal helpers kept accessible to the pub modules above but not exposed
// to downstream crates. `allow(dead_code)` silences warnings for items only
// reached by the CLI binary / MCP server.
#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod git;
#[allow(dead_code)]
mod glob;
mod outline;
#[allow(dead_code)]
mod util;
#[allow(dead_code)]
mod walk;
