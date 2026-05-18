use std::{io, path::PathBuf};

use nx_object::write::{Pfs0Builder, pfs0};

pub fn handle_subcommand(args: Args) -> Result<(), Error> {
    // Build PFS0 from directory
    let pfs0_builder =
        Pfs0Builder::from_directory(&args.in_directory).map_err(|err| Error::CollectEntries {
            path: args.in_directory.clone(),
            source: err,
        })?;

    // Build the PFS0 binary
    let pfs0_data = pfs0_builder.build().map_err(Error::BuildArchive)?;

    // Write to output file
    std::fs::write(&args.out_pfs0_filepath, &pfs0_data).map_err(|err| Error::WriteOutput {
        path: args.out_pfs0_filepath.clone(),
        source: err,
    })?;

    Ok(())
}

#[derive(clap::Args)]
pub struct Args {
    /// Path to the input directory
    pub in_directory: PathBuf,

    /// Path to the output PFS0 file
    pub out_pfs0_filepath: PathBuf,
}

/// Errors from the `build-pfs0` subcommand
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to collect entries from the input directory
    #[error("Failed to collect PFS0 entries from directory '{}'", path.display())]
    CollectEntries {
        path: PathBuf,
        source: pfs0::BuildError,
    },

    /// Failed to build the PFS0 archive
    #[error("Failed to build PFS0 archive")]
    BuildArchive(#[source] pfs0::BuildError),

    /// Failed to write the PFS0 output file to disk
    #[error("Failed to write PFS0 file '{}'", path.display())]
    WriteOutput { path: PathBuf, source: io::Error },
}
