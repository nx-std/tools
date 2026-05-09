use clap::Parser as _;
use cli::{Cargo, CargoNxSubcommand};
use tracing_subscriber::EnvFilter;

mod cli;
mod cmd;

fn main() {
    // Set up the logger
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Parse the command-line arguments and handle the subcommand
    let Cargo::Nx(args) = Cargo::parse();
    match args.subcommand {
        CargoNxSubcommand::New(args) => cmd::new::handle_subcommand(args),
        CargoNxSubcommand::Build(args) => cmd::build::handle_subcommand(args),
        CargoNxSubcommand::Link(args) => cmd::link::handle_subcommand(args),
    }
}
