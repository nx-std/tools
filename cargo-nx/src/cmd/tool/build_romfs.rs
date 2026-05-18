use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::PathBuf,
};

use nx_object::write::{RomFsBuilder, romfs};

pub fn handle_subcommand(args: Args) -> Result<(), Error> {
    // Build RomFS from directory
    let romfs_bytes = RomFsBuilder::from_directory(&args.in_directory)
        .map_err(|err| Error::CollectEntries {
            path: args.in_directory.clone(),
            source: err,
        })?
        .build()
        .map_err(Error::BuildImage)?;

    // Create output file
    let mut output = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&args.out_romfs_filepath)
        .map_err(|err| Error::CreateOutput {
            path: args.out_romfs_filepath.clone(),
            source: err,
        })?;

    // Write RomFS bytes
    output
        .write_all(&romfs_bytes)
        .map_err(|err| Error::WriteOutput {
            path: args.out_romfs_filepath.clone(),
            source: err,
        })?;

    Ok(())
}

#[derive(clap::Args)]
pub struct Args {
    /// Path to the input directory
    pub in_directory: PathBuf,

    /// Path to the output RomFS file
    pub out_romfs_filepath: PathBuf,
}

/// Errors from the `build-romfs` subcommand
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to collect entries from the input directory
    #[error("Failed to collect RomFS entries from directory '{}'", path.display())]
    CollectEntries {
        path: PathBuf,
        source: romfs::FromDirectoryError,
    },

    /// Failed to build the RomFS image
    #[error("Failed to build RomFS image")]
    BuildImage(#[source] romfs::BuildError),

    /// Failed to create the output file
    #[error("Failed to create RomFS output file '{}'", path.display())]
    CreateOutput { path: PathBuf, source: io::Error },

    /// Failed to write the RomFS data to the output file
    #[error("Failed to write RomFS file '{}'", path.display())]
    WriteOutput { path: PathBuf, source: io::Error },
}
