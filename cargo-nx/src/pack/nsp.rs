//! Build an NSP (PFS0 with `main` + `main.npdm`) entirely in memory.

use nx_object::write::{Pfs0Builder, pfs0};

/// Build an NSP image from a `main` NSO and a `main.npdm`.
///
/// The bytes are assembled into a PFS0 archive without touching the filesystem.
///
/// # Errors
///
/// Returns an error if either entry is rejected by the archive. Serializing the
/// archive itself cannot fail.
pub fn build_nsp(main_nso: Vec<u8>, main_npdm: Vec<u8>) -> Result<Vec<u8>, Error> {
    Ok(Pfs0Builder::new()
        .add_file("main", main_nso)
        .map_err(Error::AddFile)?
        .add_file("main.npdm", main_npdm)
        .map_err(Error::AddFile)?
        .build())
}

/// Errors from NSP packaging.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An entry was rejected by the archive.
    ///
    /// Both entry names are fixed by this function, so this indicates a defect
    /// here rather than bad input.
    #[error("Failed to add file to PFS0")]
    AddFile(#[source] pfs0::AddFileError),
}
