use std::{io, path::PathBuf};

use cargo_nx::npdm::{self, NpdmDescriptor};

pub fn handle_subcommand(args: Args) -> Result<(), Error> {
    let json_content = std::fs::read_to_string(&args.json_file).map_err(|err| Error::ReadJson {
        path: args.json_file.clone(),
        source: err,
    })?;

    let descriptor: NpdmDescriptor =
        serde_json::from_str(&json_content).map_err(|err| Error::ParseJson {
            path: args.json_file.clone(),
            source: err,
        })?;

    let npdm_bytes = descriptor.build().map_err(Error::Build)?;

    std::fs::write(&args.npdm_file, npdm_bytes).map_err(|err| Error::WriteNpdm {
        path: args.npdm_file.clone(),
        source: err,
    })?;

    Ok(())
}

#[derive(clap::Args)]
pub struct Args {
    /// Path to the input JSON descriptor file
    pub json_file: PathBuf,

    /// Path to the output NPDM file
    pub npdm_file: PathBuf,
}

/// Errors from the `npdmtool` subcommand
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to read the JSON descriptor file from disk
    #[error("Failed to read JSON file '{}'", path.display())]
    ReadJson { path: PathBuf, source: io::Error },

    /// Failed to deserialize the JSON descriptor file
    #[error("Failed to parse JSON file '{}'", path.display())]
    ParseJson {
        path: PathBuf,
        source: serde_json::Error,
    },

    /// Failed to build the NPDM image from the descriptor
    #[error("Failed to build NPDM from descriptor")]
    Build(#[source] npdm::Error),

    /// Failed to write the NPDM output file to disk
    #[error("Failed to write NPDM file '{}'", path.display())]
    WriteNpdm { path: PathBuf, source: io::Error },
}
