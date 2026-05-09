use std::{fs, io, path::PathBuf};

use nx_object::{
    elf::{self, ElfSegments},
    write::nso,
};

pub fn handle_subcommand(args: Args) -> Result<(), Error> {
    // Load ELF file
    let elf_data = fs::read(&args.elf_file).map_err(|err| Error::ReadElf {
        path: args.elf_file.clone(),
        source: err,
    })?;

    // Parse ELF segments
    let segments = ElfSegments::parse(&elf_data).map_err(|err| Error::ParseElf {
        path: args.elf_file.clone(),
        source: err,
    })?;

    // Build NSO from segments
    let nso_data = segments
        .into_nso_builder()
        .build()
        .map_err(Error::BuildNso)?;

    // Write output file
    fs::write(&args.nso_file, &nso_data).map_err(|err| Error::WriteNso {
        path: args.nso_file.clone(),
        source: err,
    })?;

    Ok(())
}

#[derive(clap::Args)]
pub struct Args {
    /// Path to the input ELF file
    pub elf_file: PathBuf,

    /// Path to the output NSO file
    pub nso_file: PathBuf,
}

/// Errors from the `elf2nso` subcommand
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

    /// Failed to build the NSO binary from parsed segments
    #[error("Failed to build NSO: {0}")]
    BuildNso(#[source] nso::BuildError),

    /// Failed to write the NSO output file to disk
    #[error("Failed to write NSO file '{}': {source}", path.display())]
    WriteNso { path: PathBuf, source: io::Error },
}
