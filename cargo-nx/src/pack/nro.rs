//! Build an NRO from a parsed ELF and optional asset bytes.

use nx_object::{
    elf::{self, ElfSegments},
    write::nro,
};

/// Pre-built asset bytes to embed in the NRO.
#[derive(Default)]
pub struct NroAssets {
    pub icon: Option<Vec<u8>>,
    pub nacp: Option<Vec<u8>>,
    pub romfs: Option<Vec<u8>>,
}

/// Build an NRO image from ELF bytes and optional assets.
pub fn build_nro(elf_bytes: &[u8], assets: NroAssets) -> Result<Vec<u8>, Error> {
    let segments = ElfSegments::parse(elf_bytes).map_err(Error::ParseElf)?;
    let mut builder = segments.into_nro_builder();

    if let Some(icon) = assets.icon {
        builder = builder.asset_icon(icon);
    }
    if let Some(nacp) = assets.nacp {
        builder = builder.asset_nacp(nacp);
    }
    if let Some(romfs) = assets.romfs {
        builder = builder.asset_romfs(romfs);
    }

    builder.build().map_err(Error::Build)
}

/// Errors from NRO packaging.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to parse ELF: {0}")]
    ParseElf(#[source] elf::ParseError),

    #[error("Failed to build NRO: {0}")]
    Build(#[source] nro::BuildError),
}
