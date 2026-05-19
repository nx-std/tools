use std::{io, num::ParseIntError, path::PathBuf};

use nx_object::write::{NacpBuilder, nacp};

pub fn handle_subcommand(args: Args) -> Result<(), Error> {
    // Build NACP using nx-object builder
    let mut builder = NacpBuilder::new()
        .name(args.name)
        .author(args.author)
        .version(args.version);

    if let Some(titleid_str) = args.titleid {
        // Validate titleid format: 16-digit hexadecimal
        validate_titleid(&titleid_str)?;

        // Parse titleid as u64 for application_id
        let titleid = u64::from_str_radix(&titleid_str, 16).map_err(|err| Error::ParseTitleId {
            value: titleid_str,
            source: err,
        })?;

        builder = builder.application_id(titleid);
    }

    // Build NACP bytes
    let nacp_bytes = builder.build().map_err(Error::BuildNacp)?;

    // Write to output file
    std::fs::write(&args.outfile, nacp_bytes).map_err(|err| Error::WriteOutput {
        path: args.outfile.clone(),
        source: err,
    })?;

    Ok(())
}

#[derive(clap::Args)]
pub struct Args {
    /// Create a new NACP file
    #[arg(long, required = true)]
    pub create: bool,

    /// Application name
    #[arg(required = true)]
    pub name: String,

    /// Application author
    #[arg(required = true)]
    pub author: String,

    /// Application version
    #[arg(required = true)]
    pub version: String,

    /// Output NACP file path
    #[arg(required = true)]
    pub outfile: PathBuf,

    /// Title ID
    #[arg(long, value_name = "titleID", require_equals = true)]
    pub titleid: Option<String>,
}

/// Errors from the `nacptool` subcommand
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid title ID format (must be exactly 16 hexadecimal digits)
    #[error("{0}")]
    InvalidTitleId(String),

    /// Failed to parse title ID as hexadecimal
    #[error("Failed to parse titleid '{value}' as hexadecimal")]
    ParseTitleId {
        value: String,
        source: ParseIntError,
    },

    /// Failed to build the NACP structure
    #[error("Failed to build NACP")]
    BuildNacp(#[source] nacp::BuildError),

    /// Failed to write the NACP output file to disk
    #[error("Failed to write NACP file '{}'", path.display())]
    WriteOutput { path: PathBuf, source: io::Error },
}

/// Validates titleid format.
///
/// Enforces exactly 16 hexadecimal digits, rejecting inputs with:
/// - Non-hex characters
/// - Length != 16
/// - A `0x` prefix
fn validate_titleid(titleid: &str) -> Result<(), Error> {
    // A titleid must be exactly 16 hex digits with no prefix
    if titleid.len() != 16 {
        return Err(Error::InvalidTitleId(format!(
            "Invalid titleid format: expected exactly 16 hexadecimal digits, got {} characters",
            titleid.len()
        )));
    }

    // Verify all characters are valid hex digits
    if !titleid.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::InvalidTitleId(
            "Invalid titleid format: must contain only hexadecimal digits (0-9, a-f, A-F)".into(),
        ));
    }

    Ok(())
}
