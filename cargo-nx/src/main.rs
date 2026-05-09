use clap::Parser as _;
use cli::{Cargo, CargoNxSubcommand, ToolSubcommand};
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
    let result: Result<(), String> = match args.subcommand {
        CargoNxSubcommand::New(args) => {
            cmd::new::handle_subcommand(args);
            Ok(())
        }
        CargoNxSubcommand::Build(args) => {
            cmd::build::handle_subcommand(args);
            Ok(())
        }
        CargoNxSubcommand::Link(args) => {
            cmd::link::handle_subcommand(args);
            Ok(())
        }
        CargoNxSubcommand::Tool(args) => match args.subcommand {
            ToolSubcommand::Elf2nro(args) => {
                cmd::tool::elf2nro::handle_subcommand(args).map_err(|err| err.to_string())
            }
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
