use std::{io, path::PathBuf};

use nx_object::{
    elf::{self, ElfSegments},
    write::{RomFsBuilder, nro, romfs},
};

pub fn handle_subcommand(args: Args) -> Result<(), Error> {
    // Read ELF file
    let elf_data = std::fs::read(&args.elf_file).map_err(|err| Error::ReadElf {
        path: args.elf_file.clone(),
        source: err,
    })?;

    // Parse ELF segments
    let segments = ElfSegments::parse(&elf_data).map_err(|err| Error::ParseElf {
        path: args.elf_file.clone(),
        source: err,
    })?;

    // Convert to NroBuilder with segments pre-populated
    let mut builder = segments.into_nro_builder();

    // Set aligned header flag if requested
    if args.alignedheader {
        builder = builder.flags(1);
    }

    // Add icon asset if provided
    if let Some(icon_path) = &args.icon {
        let icon_data = std::fs::read(icon_path).map_err(|err| Error::ReadIcon {
            path: icon_path.clone(),
            source: err,
        })?;
        builder = builder.asset_icon(icon_data);
    }

    // Add NACP asset if provided
    if let Some(nacp_path) = &args.nacp {
        let nacp_data = std::fs::read(nacp_path).map_err(|err| Error::ReadNacp {
            path: nacp_path.clone(),
            source: err,
        })?;
        builder = builder.asset_nacp(nacp_data);
    }

    // Handle RomFS from file
    if let Some(romfs_path) = &args.romfs {
        let romfs_data = std::fs::read(romfs_path).map_err(|err| Error::ReadRomfs {
            path: romfs_path.clone(),
            source: err,
        })?;
        builder = builder.asset_romfs(romfs_data);
    }

    // Handle RomFS from directory (single-pass serialization)
    if let Some(romfsdir_path) = &args.romfsdir {
        let romfs = RomFsBuilder::from_directory(romfsdir_path).map_err(|err| {
            Error::BuildRomfsFromDir {
                path: romfsdir_path.clone(),
                source: err,
            }
        })?;

        let romfs_data = romfs.build().map_err(Error::BuildRomfs)?;

        builder = builder.asset_romfs(romfs_data);
    }

    // Build NRO
    let nro_data = builder.build().map_err(Error::BuildNro)?;

    // Write to output file
    std::fs::write(&args.nro_file, &nro_data).map_err(|err| Error::WriteNro {
        path: args.nro_file.clone(),
        source: err,
    })?;

    Ok(())
}

#[derive(clap::Args)]
pub struct Args {
    /// Path to the input ELF file
    pub elf_file: PathBuf,

    /// Path to the output NRO file
    pub nro_file: PathBuf,

    /// Path to icon file
    #[arg(long, value_name = "iconpath", require_equals = true)]
    pub icon: Option<PathBuf>,

    /// Path to NACP control file
    #[arg(long, value_name = "control.nacp", require_equals = true)]
    pub nacp: Option<PathBuf>,

    /// Path to RomFS image file
    #[arg(
        long,
        value_name = "image",
        require_equals = true,
        conflicts_with = "romfsdir"
    )]
    pub romfs: Option<PathBuf>,

    /// Path to directory to build RomFS from
    #[arg(
        long,
        value_name = "directory",
        require_equals = true,
        conflicts_with = "romfs"
    )]
    pub romfsdir: Option<PathBuf>,

    /// Use aligned header layout
    #[arg(long)]
    pub alignedheader: bool,
}

/// Errors from the `elf2nro` subcommand
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to read the input ELF file from disk
    #[error("Failed to read ELF file '{}': {source}", path.display())]
    ReadElf { path: PathBuf, source: io::Error },

    /// Failed to parse ELF segments from the input file
    #[error("Failed to parse ELF file '{}': {source}", path.display())]
    ParseElf {
        path: PathBuf,
        source: elf::ParseError,
    },

    /// Failed to read the icon file from disk
    #[error("Failed to read icon file '{}': {source}", path.display())]
    ReadIcon { path: PathBuf, source: io::Error },

    /// Failed to read the NACP control file from disk
    #[error("Failed to read NACP file '{}': {source}", path.display())]
    ReadNacp { path: PathBuf, source: io::Error },

    /// Failed to read the RomFS image file from disk
    #[error("Failed to read RomFS file '{}': {source}", path.display())]
    ReadRomfs { path: PathBuf, source: io::Error },

    /// Failed to build RomFS from a directory
    #[error("Failed to build RomFS from directory '{}': {source}", path.display())]
    BuildRomfsFromDir {
        path: PathBuf,
        source: romfs::FromDirectoryError,
    },

    /// Failed to serialize the RomFS image
    #[error("Failed to build RomFS image: {0}")]
    BuildRomfs(#[source] romfs::BuildError),

    /// Failed to build the NRO binary from parsed segments
    #[error("Failed to build NRO: {0}")]
    BuildNro(#[source] nro::BuildError),

    /// Failed to write the NRO output file to disk
    #[error("Failed to write NRO file '{}': {source}", path.display())]
    WriteNro { path: PathBuf, source: io::Error },
}
