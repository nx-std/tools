//! Build an NPDM image from a JSON descriptor.

use std::path::{Path, PathBuf};

use cargo_nx::npdm::{self, NpdmDescriptor};

/// Build an NPDM image from a JSON descriptor file on disk.
pub fn build_npdm_from_file(json_path: &Path) -> Result<Vec<u8>, Error> {
    let json_content = std::fs::read_to_string(json_path).map_err(|err| Error::ReadJson {
        path: json_path.to_path_buf(),
        source: err,
    })?;
    let descriptor: NpdmDescriptor =
        serde_json::from_str(&json_content).map_err(|err| Error::ParseJson {
            path: json_path.to_path_buf(),
            source: err,
        })?;
    descriptor.build().map_err(Error::Build)
}

/// Build an NPDM image from an already-parsed JSON value.
pub fn build_npdm_from_value(json: &serde_json::Value) -> Result<Vec<u8>, Error> {
    let descriptor =
        <NpdmDescriptor as serde::Deserialize>::deserialize(json).map_err(Error::ParseValue)?;
    descriptor.build().map_err(Error::Build)
}

/// Errors from NPDM packaging.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to read the JSON descriptor file from disk.
    #[error("Failed to read JSON file '{}'", path.display())]
    ReadJson {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Failed to deserialize the JSON descriptor file.
    #[error("Failed to parse JSON file '{}'", path.display())]
    ParseJson {
        path: PathBuf,
        source: serde_json::Error,
    },

    /// Failed to deserialize the in-memory NPDM descriptor value.
    #[error("Failed to deserialize the NPDM descriptor")]
    ParseValue(#[source] serde_json::Error),

    /// Failed to build the NPDM image from the descriptor.
    #[error("Failed to build the NPDM image from the descriptor")]
    Build(#[source] npdm::Error),
}
