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
    #[command(about = "Switch homebrew toolchain utilities")]
    Tool(ToolArgs),
}

#[derive(clap::Args)]
pub struct ToolArgs {
    #[command(subcommand)]
    pub subcommand: ToolSubcommand,
}

#[derive(clap::Subcommand)]
pub enum ToolSubcommand {
    #[command(about = "Convert an ELF file to NRO format")]
    Elf2nro(cmd::tool::elf2nro::Args),
    #[command(about = "Convert an ELF file to NSO format")]
    Elf2nso(cmd::tool::elf2nso::Args),
    #[command(about = "Convert an ELF file to KIP format")]
    Elf2kip(cmd::tool::elf2kip::Args),
    #[command(name = "build_pfs0", about = "Build a PFS0 archive from a directory")]
    BuildPfs0(cmd::tool::build_pfs0::Args),
}
