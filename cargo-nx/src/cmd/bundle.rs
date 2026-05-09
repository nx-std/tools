//! The `bundle` subcommand: package a pre-built ELF as an NRO or NSP.
//!
//! Mirrors the CLI surface of the legacy `bundle.sh` script so it can be used
//! as a drop-in replacement, but performs all packaging steps in-process via
//! [`crate::pack`]: no child processes, no on-disk intermediates beyond the
//! final output (`--tmp-dir` is created if missing for parity, but no
//! `exefs/`, `.nso`, `.npdm`, or `.nacp` files are written).

use std::{
    io,
    path::{Path, PathBuf},
};

use nx_object::write::{RomFsBuilder, romfs};

use crate::pack;

pub fn handle_subcommand(args: Args) -> Result<(), Error> {
    // Resolve all paths to absolute paths at the boundary (parity with bundle.sh).
    let input = absolutize(&args.input).map_err(|err| Error::ResolvePath {
        flag: "--input",
        path: args.input.clone(),
        source: err,
    })?;
    let output = absolutize(&args.output).map_err(|err| Error::ResolvePath {
        flag: "--output",
        path: args.output.clone(),
        source: err,
    })?;

    // Ensure tmp dir exists (parity with `mkdir -p "$TMP_DIR"`).
    let tmp_dir = absolutize(&args.tmp_dir).map_err(|err| Error::ResolvePath {
        flag: "--tmp-dir",
        path: args.tmp_dir.clone(),
        source: err,
    })?;
    std::fs::create_dir_all(&tmp_dir).map_err(|err| Error::CreateTmpDir {
        path: tmp_dir.clone(),
        source: err,
    })?;

    // Read the input ELF once.
    let elf_data = std::fs::read(&input).map_err(|err| Error::ReadElf {
        path: input.clone(),
        source: err,
    })?;

    // Mode selection mirrors bundle.sh: `--npdm-json` present => NSP, else NRO.
    if let Some(npdm_json) = args.npdm_json.as_ref() {
        let npdm_path = absolutize(npdm_json).map_err(|err| Error::ResolvePath {
            flag: "--npdm-json",
            path: npdm_json.clone(),
            source: err,
        })?;
        bundle_nsp(&elf_data, &npdm_path, &output)
    } else {
        let assets = collect_nro_assets(&args)?;
        let nro_data = pack::nro::build_nro(&elf_data, assets).map_err(Error::BuildNro)?;
        std::fs::write(&output, &nro_data).map_err(|err| Error::WriteOutput {
            path: output,
            source: err,
        })
    }
}

fn bundle_nsp(elf_data: &[u8], npdm_path: &Path, output: &Path) -> Result<(), Error> {
    let nso_bytes = pack::nso::build_nso(elf_data).map_err(Error::BuildNso)?;
    let npdm_bytes = pack::npdm::build_npdm_from_file(npdm_path).map_err(Error::BuildNpdm)?;
    let nsp_bytes = pack::nsp::build_nsp(nso_bytes, npdm_bytes).map_err(Error::BuildNsp)?;
    std::fs::write(output, &nsp_bytes).map_err(|err| Error::WriteOutput {
        path: output.to_path_buf(),
        source: err,
    })
}

fn collect_nro_assets(args: &Args) -> Result<pack::nro::NroAssets, Error> {
    let icon = match args.icon.as_ref() {
        Some(icon_path) => {
            let path = absolutize(icon_path).map_err(|err| Error::ResolvePath {
                flag: "--icon",
                path: icon_path.clone(),
                source: err,
            })?;
            Some(std::fs::read(&path).map_err(|err| Error::ReadIcon { path, source: err })?)
        }
        None => None,
    };

    let nacp = if args.no_nacp {
        None
    } else {
        let name = args.name.clone().ok_or(Error::MissingNacpField("--name"))?;
        let author = args
            .author
            .clone()
            .ok_or(Error::MissingNacpField("--author"))?;
        let version = args
            .version
            .clone()
            .ok_or(Error::MissingNacpField("--version"))?;
        Some(pack::nacp::build_simple(name, author, version).map_err(Error::BuildNacp)?)
    };

    let romfs = match args.romfs.as_ref() {
        Some(romfs_path) => {
            let path = absolutize(romfs_path).map_err(|err| Error::ResolvePath {
                flag: "--romfs",
                path: romfs_path.clone(),
                source: err,
            })?;
            let bytes = RomFsBuilder::from_directory(&path)
                .map_err(|err| Error::BuildRomfsFromDir {
                    path: path.clone(),
                    source: err,
                })?
                .build()
                .map_err(Error::BuildRomfs)?;
            Some(bytes)
        }
        None => None,
    };

    Ok(pack::nro::NroAssets { icon, nacp, romfs })
}

/// Resolve a (possibly relative) path to an absolute one, matching the
/// `make_absolute_path` helper in the legacy `bundle.sh`. The parent directory
/// must already exist; the leaf may be missing (for output paths).
fn absolutize(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    let leaf = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path '{}' has no file name component", path.display()),
        )
    })?;
    let parent_abs = match parent {
        Some(parent) => std::fs::canonicalize(parent)?,
        None => std::env::current_dir()?,
    };
    Ok(parent_abs.join(leaf))
}

#[derive(clap::Args)]
pub struct Args {
    /// Output directory (accepted for compatibility with `bundle.sh`; ignored).
    #[arg(long = "out-dir", value_name = "DIR")]
    pub out_dir: Option<PathBuf>,

    /// Path to the input ELF file.
    #[arg(long, value_name = "ELF")]
    pub input: PathBuf,

    /// Path to the output NRO or NSP file.
    #[arg(long, value_name = "FILE")]
    pub output: PathBuf,

    /// Working directory for intermediates (created if missing).
    #[arg(long = "tmp-dir", value_name = "DIR")]
    pub tmp_dir: PathBuf,

    /// Skip NACP generation (NRO mode only).
    #[arg(long = "no-nacp")]
    pub no_nacp: bool,

    /// NACP application name (NRO mode, required unless `--no-nacp`).
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,

    /// NACP author (NRO mode, required unless `--no-nacp`).
    #[arg(long, value_name = "AUTHOR")]
    pub author: Option<String>,

    /// NACP version (NRO mode, required unless `--no-nacp`).
    #[arg(long, value_name = "VERSION")]
    pub version: Option<String>,

    /// Optional icon JPEG (NRO mode).
    #[arg(long, value_name = "FILE")]
    pub icon: Option<PathBuf>,

    /// Optional RomFS source directory (NRO mode).
    #[arg(long, value_name = "DIR")]
    pub romfs: Option<PathBuf>,

    /// NPDM JSON descriptor; presence selects NSP mode.
    #[arg(long = "npdm-json", value_name = "FILE")]
    pub npdm_json: Option<PathBuf>,
}

/// Errors from the `bundle` subcommand.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to resolve {flag} path '{}': {source}", path.display())]
    ResolvePath {
        flag: &'static str,
        path: PathBuf,
        source: io::Error,
    },

    #[error("Failed to create tmp dir '{}': {source}", path.display())]
    CreateTmpDir { path: PathBuf, source: io::Error },

    #[error("Failed to read ELF file '{}': {source}", path.display())]
    ReadElf { path: PathBuf, source: io::Error },

    #[error("Failed to read icon file '{}': {source}", path.display())]
    ReadIcon { path: PathBuf, source: io::Error },

    #[error("Missing required NACP field: {0} (pass it or use --no-nacp)")]
    MissingNacpField(&'static str),

    #[error("Failed to build RomFS from directory '{}': {source}", path.display())]
    BuildRomfsFromDir {
        path: PathBuf,
        source: romfs::FromDirectoryError,
    },

    #[error("Failed to build RomFS image: {0}")]
    BuildRomfs(#[source] romfs::BuildError),

    #[error(transparent)]
    BuildNacp(pack::nacp::Error),

    #[error(transparent)]
    BuildNro(pack::nro::Error),

    #[error(transparent)]
    BuildNso(pack::nso::Error),

    #[error(transparent)]
    BuildNpdm(pack::npdm::Error),

    #[error(transparent)]
    BuildNsp(pack::nsp::Error),

    #[error("Failed to write output file '{}': {source}", path.display())]
    WriteOutput { path: PathBuf, source: io::Error },
}
