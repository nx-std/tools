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
    #[command(
        about = "Convert an ELF file to NRO format",
        after_help = include_str!("cmd/tool/elf2nro__after_help.md")
    )]
    Elf2nro(cmd::tool::elf2nro::Args),
    #[command(
        about = "Convert an ELF file to NSO format",
        after_help = include_str!("cmd/tool/elf2nso__after_help.md")
    )]
    Elf2nso(cmd::tool::elf2nso::Args),
    #[command(
        about = "Convert an ELF file to KIP format",
        after_help = include_str!("cmd/tool/elf2kip__after_help.md")
    )]
    Elf2kip(cmd::tool::elf2kip::Args),
    #[command(
        name = "build_pfs0",
        about = "Build a PFS0 archive from a directory",
        after_help = include_str!("cmd/tool/build_pfs0__after_help.md")
    )]
    BuildPfs0(cmd::tool::build_pfs0::Args),
    #[command(
        name = "build_romfs",
        about = "Build a RomFS image from a directory",
        after_help = include_str!("cmd/tool/build_romfs__after_help.md")
    )]
    BuildRomfs(cmd::tool::build_romfs::Args),
    #[command(
        about = "Convert JSON metadata to NPDM format",
        after_help = include_str!("cmd/tool/npdmtool__after_help.md")
    )]
    Npdmtool(cmd::tool::npdmtool::Args),
    #[command(
        about = "Create or manipulate NACP files",
        after_help = include_str!("cmd/tool/nacptool__after_help.md")
    )]
    Nacptool(cmd::tool::nacptool::Args),
    #[command(
        name = "bin2s",
        about = "Convert binary files to GAS assembly source",
        after_help = include_str!("cmd/tool/bin2s__after_help.md")
    )]
    Bin2s(cmd::tool::bin2s::Args),
    #[command(
        name = "bin2c",
        alias = "raw2c",
        about = "Convert binary files to C source",
        after_help = include_str!("cmd/tool/bin2c__after_help.md")
    )]
    Bin2c(cmd::tool::bin2c::Args),
}
