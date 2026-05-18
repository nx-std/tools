//! Build an NSP (PFS0 with `main` + `main.npdm`) entirely in memory.

use nx_object::write::{Pfs0Builder, pfs0};

/// Build an NSP image from a `main` NSO and a `main.npdm`.
///
/// The bytes are assembled into a PFS0 archive without touching the filesystem.
pub fn build_nsp(main_nso: Vec<u8>, main_npdm: Vec<u8>) -> Result<Vec<u8>, Error> {
    Pfs0Builder::new()
        .add_file("main", main_nso)
        .map_err(Error::AddFile)?
        .add_file("main.npdm", main_npdm)
        .map_err(Error::AddFile)?
        .build()
        .map_err(Error::Build)
}

/// Errors from NSP packaging.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to add file to PFS0")]
    AddFile(#[source] pfs0::BuildError),

    #[error("Failed to build PFS0")]
    Build(#[source] pfs0::BuildError),
}
