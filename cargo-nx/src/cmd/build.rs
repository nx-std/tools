//! The `build` subcommand: compile a project for the Switch and pack the result.
//!
//! Runs `cargo build` against the Switch target and reads its JSON message stream
//! to learn which artifact was produced, then packs that artifact into the format
//! the package's `[package.metadata.nx]` block asks for.
//!
//! The output format is declared per package, and a package may declare only one:
//! `nx.nro` and `nx.nsp` together are rejected rather than resolved.
//!
//! This module owns the orchestration only. What a manifest may say lives in
//! [`metadata`], and turning what it says into format inputs lives in [`nacp`] and
//! [`npdm`], so a change to either format leaves the build loop untouched.

use std::{
    io::{self, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use cargo_metadata::{Artifact, Message, MetadataCommand, Package};
use nx_object::write::{RomFsBuilder, romfs};

mod metadata;
mod nacp;
mod npdm;

use self::{
    metadata::{NroMetadata, NspMetadata},
    nacp::{BuildNacpError, build_nacp_from_metadata},
    npdm::ConvertNpdmError,
};
use crate::{pack, ui};

/// The default target triple to use when building.
const DEFAULT_TARGET_TRIPLE: &str = "aarch64-nintendo-switch-freestanding";

/// The default icon to use when building an NRO.
const DEFAULT_NRO_ICON: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/nro/default_icon.jpg"
));

/// Handle the `build` subcommand.
///
/// # Errors
///
/// Returns an error if workspace metadata cannot be read, if the package named by
/// `--package` is absent or declares more than one output format, if `cargo build`
/// cannot be spawned or exits non-zero, or if the artifact it produced cannot be
/// read and packed. A non-zero `cargo build` status is propagated as the command's
/// own exit code rather than collapsed to a generic failure.
pub fn handle_subcommand(args: Args) -> Result<(), Error> {
    let metadata = MetadataCommand::new()
        .manifest_path("./Cargo.toml")
        .no_deps()
        .exec()
        .map_err(Error::Metadata)?;

    let rust_target_path = match std::env::var("RUST_TARGET_PATH") {
        Ok(path) => PathBuf::from(path),
        Err(_) => metadata.workspace_root.clone().into_std_path_buf(),
    };

    let target = args.target.as_deref().unwrap_or(DEFAULT_TARGET_TRIPLE);
    if args.verbose {
        ui::status("Target", target);
    }

    let Some(build_target_path) = rust_target_path.to_str() else {
        return Err(Error::NonUtf8TargetPath {
            path: rust_target_path,
        });
    };
    if args.verbose {
        ui::status("Workspace", build_target_path);
    }

    let mut build_args: Vec<String> = vec![
        String::from("build"),
        format!("--target={target}"),
        String::from("--message-format=json-diagnostic-rendered-ansi"),
    ];
    if args.release {
        build_args.push(String::from("--release"));
    }

    let build_crates: Vec<Package> = match args.package {
        Some(target_package) => {
            let Some(package) = metadata
                .packages
                .iter()
                .find(|needle| needle.name == target_package)
            else {
                return Err(Error::PackageNotFound {
                    name: target_package,
                });
            };
            vec![package.clone()]
        }
        None => metadata.packages.to_vec(),
    };

    for build_crate in build_crates {
        let mut build_args = build_args.clone();
        build_args.extend_from_slice(&[String::from("-p"), build_crate.name.to_string()]);
        if args.all_features {
            build_args.push("--all-features".to_string());
        }

        if let Some(features) = args.features.as_ref() {
            build_args.extend_from_slice(&[String::from("--features"), features.clone()]);
        }

        let metadata_v = build_crate.metadata;

        let is_nsp = metadata_v.pointer("/nx/nsp").is_some();
        let is_nro = metadata_v.pointer("/nx/nro").is_some();
        if is_nsp && is_nro {
            return Err(Error::MultipleFormats {
                package: build_crate.name.to_string(),
            });
        } else if is_nsp {
            ui::status("Building", "NSP package");
        } else if is_nro {
            ui::status("Building", "NRO artifact");
        } else {
            ui::status("Building", build_crate.name.as_ref());
        }

        let mut command = Command::new("cargo")
            .args(&build_args)
            .stdout(Stdio::piped())
            .env("RUST_TARGET_PATH", build_target_path)
            .spawn()
            .map_err(Error::CargoSpawn)?;

        // SAFETY: stdout was configured as `Stdio::piped()`, so it is always `Some`.
        let stdout = command.stdout.take().expect("cargo stdout was piped");
        for message in Message::parse_stream(BufReader::new(stdout)) {
            match message.map_err(Error::MessageParse)? {
                Message::CompilerArtifact(ref artifact)
                    if (artifact.target.kind.contains(&"bin".into())
                        || artifact.target.kind.contains(&"cdylib".into())) =>
                {
                    let Some(package) = metadata
                        .packages
                        .iter()
                        .find(|needle| needle.id == artifact.package_id)
                    else {
                        continue;
                    };

                    let Some(root) = package.manifest_path.parent() else {
                        continue;
                    };
                    let root = root.as_std_path();

                    if is_nsp {
                        if let Some(nsp_json) = metadata_v.pointer("/nx/nsp").cloned() {
                            let nsp_metadata: NspMetadata =
                                serde_json::from_value(nsp_json).unwrap_or_default();
                            handle_nsp_format(root, artifact, nsp_metadata)?;
                        }
                    } else if is_nro && let Some(nro_json) = metadata_v.pointer("/nx/nro").cloned()
                    {
                        let nro_metadata: NroMetadata =
                            serde_json::from_value(nro_json).unwrap_or_default();
                        handle_nro_format(root, artifact, nro_metadata)?;
                    }
                }
                Message::CompilerMessage(msg) => match msg.message.rendered {
                    Some(rendered) => ui::raw(&rendered),
                    None => ui::raw(&format!("{msg:?}\n")),
                },
                _ => {}
            }
        }

        let status = command.wait().map_err(Error::CargoWait)?;
        if !status.success() {
            return Err(Error::CargoBuildFailed {
                code: status.code().unwrap_or(1),
            });
        }
    }

    Ok(())
}

/// The `build` subcommand CLI arguments.
#[derive(clap::Args)]
pub struct Args {
    /// Builds using the release profile.
    #[arg(short, long)]
    pub release: bool,
    /// The package name of the project to build.
    #[arg(short, long, value_name = "DIR", value_parser)]
    pub package: Option<String>,
    /// The custom target triple to use, if any.
    #[arg(short, long)]
    pub target: Option<String>,
    /// Displays extra information during the build process.
    #[arg(short, long)]
    pub verbose: bool,
    /// Passes on the requested features to `cargo build`
    #[arg(long, value_parser)]
    pub features: Option<String>,
    /// Passes the `all-features` flag to `cargo build`
    #[arg(long)]
    pub all_features: bool,
}

/// Errors from the `build` subcommand.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to load Cargo metadata for the workspace.
    #[error("Failed to read Cargo metadata")]
    Metadata(#[source] cargo_metadata::Error),

    /// The resolved Rust target path is not valid UTF-8.
    #[error("Target path is not valid UTF-8: '{}'", path.display())]
    NonUtf8TargetPath { path: PathBuf },

    /// The package requested with `--package` is not in the workspace.
    #[error("Package '{name}' not found in the workspace")]
    PackageNotFound { name: String },

    /// A crate declares both `nx.nro` and `nx.nsp` output formats.
    #[error("Package '{package}' declares multiple target formats, which is not yet supported")]
    MultipleFormats { package: String },

    /// The `cargo build` child process could not be spawned.
    #[error("Failed to spawn `cargo build`")]
    CargoSpawn(#[source] io::Error),

    /// A message from the `cargo build` JSON stream could not be parsed.
    #[error("Failed to parse `cargo build` output")]
    MessageParse(#[source] io::Error),

    /// Waiting for the `cargo build` child process to exit failed.
    #[error("Failed to wait for `cargo build`")]
    CargoWait(#[source] io::Error),

    /// The `cargo build` child process exited with a non-zero status.
    #[error("`cargo build` failed with exit code {code}")]
    CargoBuildFailed { code: i32 },

    /// The compiled ELF artifact could not be read from disk.
    #[error("Failed to read ELF file '{}'", path.display())]
    ReadElf { path: PathBuf, source: io::Error },

    /// The RomFS directory could not be collected into an image.
    #[error("Failed to build RomFS from directory '{}'", path.display())]
    BuildRomfsFromDir {
        path: PathBuf,
        source: romfs::FromDirectoryError,
    },

    /// The RomFS image could not be serialized.
    #[error("Failed to build RomFS image")]
    BuildRomfs(#[source] romfs::BuildError),

    /// The icon file could not be read from disk.
    #[error("Failed to read icon file '{}'", path.display())]
    ReadIcon { path: PathBuf, source: io::Error },

    /// The NACP control data could not be built from `Cargo.toml` metadata.
    #[error("Failed to build NACP from metadata")]
    BuildNacp(#[source] BuildNacpError),

    /// The NRO artifact could not be assembled.
    #[error("Failed to assemble the NRO artifact")]
    BuildNro(#[source] pack::nro::Error),

    /// The inline NPDM metadata could not be converted to descriptor JSON.
    #[error("Failed to convert inline NPDM metadata")]
    ConvertNpdm(#[source] ConvertNpdmError),

    /// The crate declares neither `npdm` nor `npdm_json` for its NSP build.
    #[error("NSP build requires `npdm` or `npdm_json` under `nx.nsp`")]
    MissingNpdm,

    /// The process metadata (NPDM) could not be built.
    #[error("Failed to build process metadata")]
    BuildNpdm(#[source] pack::npdm::Error),

    /// The NSO image could not be assembled.
    #[error("Failed to assemble the NSO image")]
    BuildNso(#[source] pack::nso::Error),

    /// The NSP package could not be assembled.
    #[error("Failed to assemble the NSP package")]
    BuildNsp(#[source] pack::nsp::Error),

    /// The packaged artifact could not be written to disk.
    #[error("Failed to write output file '{}'", path.display())]
    WriteOutput { path: PathBuf, source: io::Error },
}

impl ui::CliError for Error {
    fn exit_code(&self) -> i32 {
        match self {
            // Propagate the underlying `cargo build` exit code.
            Self::CargoBuildFailed { code } => *code,
            _ => ui::EXIT_FAILURE,
        }
    }
}

fn handle_nro_format(root: &Path, artifact: &Artifact, metadata: NroMetadata) -> Result<(), Error> {
    let elf = artifact.filenames[0].clone();
    let nro = artifact_path_with_extension(
        artifact,
        if metadata.overlay == Some(true) {
            "ovl"
        } else {
            "nro"
        },
    );

    // Read the compiled ELF
    let elf_data = std::fs::read(elf.as_std_path()).map_err(|err| Error::ReadElf {
        path: elf.into_std_path_buf(),
        source: err,
    })?;

    // Build RomFS bytes if a directory is specified
    let romfs_bytes = match metadata.romfs.as_ref() {
        Some(romfs_dir) => {
            let romfs_path = root.join(romfs_dir);
            let builder = RomFsBuilder::from_directory(&romfs_path).map_err(|err| {
                Error::BuildRomfsFromDir {
                    path: romfs_path,
                    source: err,
                }
            })?;
            Some(builder.build().map_err(Error::BuildRomfs)?)
        }
        None => None,
    };

    // Resolve the icon (user-provided or default)
    let icon_bytes = match metadata.icon.as_ref() {
        Some(icon_file) => {
            let icon_path = root.join(icon_file);
            std::fs::read(&icon_path).map_err(|err| Error::ReadIcon {
                path: icon_path,
                source: err,
            })?
        }
        None => DEFAULT_NRO_ICON.to_vec(),
    };

    // Build NACP if specified
    let nacp_bytes = match metadata.nacp.as_ref() {
        Some(nacp_metadata) => {
            Some(build_nacp_from_metadata(nacp_metadata).map_err(Error::BuildNacp)?)
        }
        None => None,
    };

    let nro_data = pack::nro::build_nro(
        &elf_data,
        pack::nro::NroAssets {
            icon: Some(icon_bytes),
            nacp: nacp_bytes,
            romfs: romfs_bytes,
        },
    )
    .map_err(Error::BuildNro)?;

    // Write the NRO output
    std::fs::write(&nro, &nro_data).map_err(|err| Error::WriteOutput {
        path: nro.clone(),
        source: err,
    })?;

    ui::status("Built", &nro.to_string_lossy());
    Ok(())
}

fn handle_nsp_format(root: &Path, artifact: &Artifact, metadata: NspMetadata) -> Result<(), Error> {
    let elf = artifact.filenames[0].clone();
    let exefs_nsp = artifact_path_with_extension(artifact, "nsp");

    // Build NPDM bytes (from inline TOML or external JSON file)
    let npdm_bytes = if let Some(inline_npdm) = metadata.npdm {
        let descriptor = npdm::to_descriptor(&inline_npdm).map_err(Error::ConvertNpdm)?;
        pack::npdm::build_npdm_from_descriptor(descriptor).map_err(Error::BuildNpdm)?
    } else if let Some(npdm_json) = metadata.npdm_json {
        let npdm_json_path = root.join(npdm_json);
        pack::npdm::build_npdm_from_file(&npdm_json_path).map_err(Error::BuildNpdm)?
    } else {
        return Err(Error::MissingNpdm);
    };

    // Build the NSO from the compiled ELF
    let elf_data = std::fs::read(elf.as_std_path()).map_err(|err| Error::ReadElf {
        path: elf.into_std_path_buf(),
        source: err,
    })?;
    let nso_data = pack::nso::build_nso(&elf_data).map_err(Error::BuildNso)?;

    // Assemble the NSP (PFS0) in memory
    let nsp_data = pack::nsp::build_nsp(nso_data, npdm_bytes).map_err(Error::BuildNsp)?;
    std::fs::write(&exefs_nsp, &nsp_data).map_err(|err| Error::WriteOutput {
        path: exefs_nsp.clone(),
        source: err,
    })?;

    ui::status("Built", &exefs_nsp.to_string_lossy());
    Ok(())
}

/// The path `artifact`'s compiled ELF occupies, with its extension replaced by
/// `extension` — where the packed output for that container format is written.
fn artifact_path_with_extension(artifact: &Artifact, extension: &str) -> PathBuf {
    let mut elf = artifact.filenames[0].clone();
    // A compiler artifact filename always has a file name, so this succeeds.
    elf.set_extension(extension);
    elf.into_std_path_buf()
}
#[cfg(test)]
mod tests {
    use super::Error;
    use crate::ui::{CliError as _, EXIT_FAILURE};

    #[test]
    fn exit_code_for_cargo_build_failure_propagates_the_child_status() {
        //* Given
        let err = Error::CargoBuildFailed { code: 101 };

        //* When
        let code = err.exit_code();

        //* Then
        assert_eq!(code, 101);
    }

    #[test]
    fn exit_code_for_other_errors_defaults_to_failure() {
        //* Given
        let err = Error::MissingNpdm;

        //* When
        let code = err.exit_code();

        //* Then
        assert_eq!(code, EXIT_FAILURE);
    }
}
