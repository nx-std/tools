//! Build an NPDM image from a JSON descriptor.

use std::path::{Path, PathBuf};

use nx_object::write::npdm::NpdmBuilder;

use crate::cmd::tool::npdmtool::{self, parse_npdm_json_value};

/// Build an NPDM image from a JSON descriptor file on disk.
pub fn build_npdm_from_file(json_path: &Path) -> Result<Vec<u8>, Error> {
    let json_content = std::fs::read_to_string(json_path).map_err(|err| Error::ReadJson {
        path: json_path.to_path_buf(),
        source: err,
    })?;
    let json: serde_json::Value =
        serde_json::from_str(&json_content).map_err(|err| Error::ParseJson {
            path: json_path.to_path_buf(),
            source: err,
        })?;
    build_npdm_from_value(&json)
}

/// Build an NPDM image from an already-parsed JSON value.
pub fn build_npdm_from_value(json: &serde_json::Value) -> Result<Vec<u8>, Error> {
    let (metadata, aci, acid) = parse_npdm_json_value(json).map_err(Error::Parse)?;
    Ok(NpdmBuilder::new(metadata)
        .with_aci(aci)
        .with_acid(acid)
        .build())
}

/// Errors from NPDM packaging.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to read JSON file '{}': {source}", path.display())]
    ReadJson {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse JSON file '{}': {source}", path.display())]
    ParseJson {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("Failed to parse NPDM descriptor: {0}")]
    Parse(#[source] npdmtool::Error),
}
