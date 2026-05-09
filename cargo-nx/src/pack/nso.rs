//! Build an NSO from a parsed ELF.

use nx_object::{
    elf::{self, ElfSegments},
    write::nso,
};

/// Build an NSO image from ELF bytes.
pub fn build_nso(elf_bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let segments = ElfSegments::parse(elf_bytes).map_err(Error::ParseElf)?;
    segments.into_nso_builder().build().map_err(Error::Build)
}

/// Errors from NSO packaging.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to parse ELF: {0}")]
    ParseElf(#[source] elf::ParseError),

    #[error("Failed to build NSO: {0}")]
    Build(#[source] nso::BuildError),
}
