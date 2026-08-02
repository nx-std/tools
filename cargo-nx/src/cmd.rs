//! The `cargo nx` subcommands.
//!
//! Each module owns one subcommand and exposes the same pair: a `clap`-derived
//! `Args` and a `handle_subcommand` that consumes it. Each declares its own
//! `Error` implementing [`crate::ui::CliError`], so `main` can report any of them
//! and take an exit code without knowing which subcommand ran.

pub mod build;
pub mod bundle;
pub mod link;
pub mod new;
pub mod tool;
