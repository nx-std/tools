//! CLI structure for cargo-nx

use crate::cmd;

#[derive(clap::Parser)]
#[clap(name = "cargo", bin_name = "cargo")]
pub enum Cargo {
    Nx(CargoNxArgs),
}

#[derive(clap::Args)]
#[clap(author, version, about)]
pub struct CargoNxArgs {
    #[command(subcommand)]
    pub subcommand: CargoNxSubcommand,
}

#[derive(clap::Subcommand)]
pub enum CargoNxSubcommand {
    #[command(about = "Create a new Rust project for the Nintendo Switch")]
    New(cmd::new::Args),
    #[command(about = "Build a Rust project for the Nintendo Switch")]
    Build(cmd::build::Args),
    #[command(about = "Send a file to the Nintendo Switch")]
    Link(cmd::link::Args),
}
