//! Build NACP control bytes from name/author/version inputs.

use nx_object::write::{NacpBuilder, nacp};

/// Build NACP bytes from a flat name/author/version triple.
///
/// Used by `cargo nx bundle` (and the legacy `bundle.sh` script): all 16 NACP
/// languages receive the same name/author. No title id is set.
pub fn build_simple(name: String, author: String, version: String) -> Result<Vec<u8>, Error> {
    NacpBuilder::new()
        .name(name)
        .author(author)
        .version(version)
        .build()
        .map_err(Error::Build)
}

/// Errors from NACP packaging.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to build NACP: {0}")]
    Build(#[source] nacp::BuildError),
}
