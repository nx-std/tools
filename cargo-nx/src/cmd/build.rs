use std::{
    io::{self, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use cargo_metadata::{Artifact, Message, MetadataCommand, Package};
use nx_object::{
    read::SetLanguage,
    write::{NacpBuilder, RomFsBuilder, romfs},
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
    #[error("Failed to build NACP from metadata: {0}")]
    BuildNacp(String),

    /// The NRO artifact could not be assembled.
    #[error("Failed to assemble the NRO artifact")]
    BuildNro(#[source] pack::nro::Error),

    /// The inline NPDM metadata could not be converted to descriptor JSON.
    #[error("Failed to convert inline NPDM metadata: {0}")]
    ConvertNpdm(String),

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

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct NspMetadata {
    npdm_json: Option<String>,
    npdm: Option<InlineNpdm>,
}

/// Inline NPDM metadata structure matching Cargo.toml format
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct InlineNpdm {
    name: String,
    main_thread_stack_size: String,
    main_thread_priority: u8,
    #[serde(alias = "main_thread_core_number", alias = "default_cpu_id")]
    main_thread_core_number: u8,
    #[serde(default)]
    version: u32,
    address_space_type: u8,
    is_64_bit: bool,
    optimize_memory_allocation: bool,
    disable_device_address_space_merge: bool,
    #[serde(default = "default_is_retail")]
    is_retail: bool,
    #[serde(alias = "title_id")]
    program_id: String,
    #[serde(default)]
    fs_access_control: Option<InlineFsAccessControl>,
    #[serde(default)]
    service_access_control: Option<InlineServiceAccessControl>,
    #[serde(default)]
    kernel_capabilities: Option<InlineKernelCapabilities>,
}

fn default_is_retail() -> bool {
    true
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct InlineFsAccessControl {
    flags: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct InlineServiceAccessControl {
    #[serde(default)]
    accessed_services: Vec<String>,
    #[serde(default)]
    hosted_services: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct InlineKernelCapabilities {
    #[serde(default)]
    highest_priority: Option<u8>,
    #[serde(default)]
    lowest_priority: Option<u8>,
    #[serde(default)]
    max_core_number: Option<u8>,
    #[serde(default)]
    min_core_number: Option<u8>,
    #[serde(default)]
    enable_system_calls: Vec<String>,
    #[serde(default)]
    kernel_version: Option<String>,
}

/// Serde-compatible NACP metadata that deserializes from Cargo.toml
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct NacpMetadata {
    name: Option<String>,
    author: Option<String>,
    version: Option<String>,
    title_id: Option<String>,
    dlc_base_title_id: Option<String>,
    lang: Option<NacpLangEntries>,
}

/// Per-language NACP entries.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct NacpLangEntries {
    #[serde(rename = "en-US")]
    en_us: Option<NacpLangEntry>,
    #[serde(rename = "en-GB")]
    en_gb: Option<NacpLangEntry>,
    ja: Option<NacpLangEntry>,
    fr: Option<NacpLangEntry>,
    de: Option<NacpLangEntry>,
    #[serde(rename = "es-419")]
    es_419: Option<NacpLangEntry>,
    es: Option<NacpLangEntry>,
    it: Option<NacpLangEntry>,
    nl: Option<NacpLangEntry>,
    #[serde(rename = "fr-CA")]
    fr_ca: Option<NacpLangEntry>,
    pt: Option<NacpLangEntry>,
    ru: Option<NacpLangEntry>,
    ko: Option<NacpLangEntry>,
    #[serde(rename = "zh-TW")]
    zh_tw: Option<NacpLangEntry>,
    #[serde(rename = "zh-CN")]
    zh_cn: Option<NacpLangEntry>,
    #[serde(rename = "pt-BR")]
    pt_br: Option<NacpLangEntry>,
}

/// Single language entry with name and author
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct NacpLangEntry {
    name: String,
    author: String,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct NroMetadata {
    romfs: Option<String>,
    icon: Option<String>,
    nacp: Option<NacpMetadata>,
    overlay: Option<bool>,
}

fn handle_nro_format(root: &Path, artifact: &Artifact, metadata: NroMetadata) -> Result<(), Error> {
    let elf = artifact.filenames[0].clone();
    let nro = get_output_elf_path_as(
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
    let exefs_nsp = get_output_elf_path_as(artifact, "nsp");

    // Build NPDM bytes (from inline TOML or external JSON file)
    let npdm_bytes = if let Some(inline_npdm) = metadata.npdm {
        let json_value = convert_inline_npdm_to_json(&inline_npdm).map_err(Error::ConvertNpdm)?;
        pack::npdm::build_npdm_from_value(&json_value).map_err(Error::BuildNpdm)?
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

/// Convert NACP metadata from Cargo.toml to NACP bytes using nx-object
///
/// NACP serialization behavior:
/// - Uses global name/author/version as defaults for all languages
/// - Per-language entries override the global defaults
/// - Parses title_id and dlc_base_title_id from hex strings
/// - Applies default values for unset fields
fn build_nacp_from_metadata(metadata: &NacpMetadata) -> Result<Vec<u8>, String> {
    // Use defaults for unset fields
    let default_name = metadata
        .name
        .clone()
        .unwrap_or_else(|| "Unknown Application".to_string());
    let default_author = metadata
        .author
        .clone()
        .unwrap_or_else(|| "Unknown Author".to_string());
    let default_version = metadata
        .version
        .clone()
        .unwrap_or_else(|| "1.0.0".to_string());

    let mut builder = NacpBuilder::new().version(default_version);

    // If per-language entries exist, use them; otherwise use global defaults
    match &metadata.lang {
        Some(lang_entries) => {
            // Set per-language entries, falling back to the global defaults
            let fallback = NacpLangEntry {
                name: default_name.clone(),
                author: default_author.clone(),
            };

            // Helper macro to set language entry with fallback
            macro_rules! set_lang {
                ($lang:expr, $entry:expr) => {
                    if let Some(ref entry) = $entry {
                        builder = builder
                            .name_for_language($lang, &entry.name)
                            .author_for_language($lang, &entry.author);
                    } else {
                        builder = builder
                            .name_for_language($lang, &fallback.name)
                            .author_for_language($lang, &fallback.author);
                    }
                };
            }

            set_lang!(SetLanguage::ENUS, lang_entries.en_us);
            set_lang!(SetLanguage::ENGB, lang_entries.en_gb);
            set_lang!(SetLanguage::JA, lang_entries.ja);
            set_lang!(SetLanguage::FR, lang_entries.fr);
            set_lang!(SetLanguage::DE, lang_entries.de);
            set_lang!(SetLanguage::ES419, lang_entries.es_419);
            set_lang!(SetLanguage::ES, lang_entries.es);
            set_lang!(SetLanguage::IT, lang_entries.it);
            set_lang!(SetLanguage::NL, lang_entries.nl);
            set_lang!(SetLanguage::FRCA, lang_entries.fr_ca);
            set_lang!(SetLanguage::PT, lang_entries.pt);
            set_lang!(SetLanguage::RU, lang_entries.ru);
            set_lang!(SetLanguage::KO, lang_entries.ko);
            set_lang!(SetLanguage::ZHTW, lang_entries.zh_tw);
            set_lang!(SetLanguage::ZHCN, lang_entries.zh_cn);
            set_lang!(SetLanguage::PTBR, lang_entries.pt_br);
        }
        None => {
            // No per-language entries, use global defaults for all languages
            builder = builder.name(default_name).author(default_author);
        }
    }

    // Parse title_id if provided
    if let Some(ref title_id_str) = metadata.title_id {
        let title_id = u64::from_str_radix(title_id_str, 16)
            .map_err(|err| format!("Invalid title_id '{}': {}", title_id_str, err))?;
        builder = builder.application_id(title_id);
    }

    // Note: dlc_base_title_id is not supported by NacpBuilder yet.
    // NacpBuilder already sets add_on_content_base_id to title_id + 0x1000
    // automatically, so we don't need to handle it separately.

    // Build NACP bytes
    builder
        .build()
        .map_err(|err| format!("Failed to build NACP: {}", err))
}

fn get_output_elf_path_as(artifact: &Artifact, extension: &str) -> PathBuf {
    let mut elf = artifact.filenames[0].clone();
    // A compiler artifact filename always has a file name, so this succeeds.
    elf.set_extension(extension);
    elf.into_std_path_buf()
}

fn convert_inline_npdm_to_json(inline: &InlineNpdm) -> Result<serde_json::Value, String> {
    // Strip `0x` prefixes from hex values (parser expects bare hex digits)
    let program_id = strip_hex_prefix(&inline.program_id);
    let main_thread_stack_size = strip_hex_prefix(&inline.main_thread_stack_size);

    // Format version as hex string (parser expects hex string, not JSON number)
    let version = format!("{:x}", inline.version);

    let mut json = serde_json::json!({
        "name": inline.name,
        "main_thread_stack_size": main_thread_stack_size,
        "main_thread_priority": inline.main_thread_priority,
        "default_cpu_id": inline.main_thread_core_number,
        "version": version,
        "address_space_type": inline.address_space_type,
        "is_64_bit": inline.is_64_bit,
        "optimize_memory_allocation": inline.optimize_memory_allocation,
        "disable_device_address_space_merge": inline.disable_device_address_space_merge,
        "program_id": program_id,
        // ACID required fields
        "is_retail": inline.is_retail,
        "pool_partition": 0,
        "program_id_range_min": program_id,
        "program_id_range_max": program_id,
    });

    // Add filesystem_access (always required by parser)
    if let Some(ref fs_access) = inline.fs_access_control {
        // Strip `0x` prefix from permissions hex string
        let permissions = strip_hex_prefix(&fs_access.flags);
        json["filesystem_access"] = serde_json::json!({
            "permissions": permissions
        });
    } else {
        // Emit default value when not specified (parser requires this field)
        json["filesystem_access"] = serde_json::json!({
            "permissions": "0"
        });
    }

    // Add service_access if present (emit string arrays, not objects)
    if let Some(ref svc_access) = inline.service_access_control {
        // Parser expects string arrays for service_access and service_host
        if !svc_access.accessed_services.is_empty() {
            json["service_access"] = serde_json::json!(svc_access.accessed_services);
        }

        if !svc_access.hosted_services.is_empty() {
            json["service_host"] = serde_json::json!(svc_access.hosted_services);
        }
    }

    // Add kernel_capabilities (always required by parser)
    json["kernel_capabilities"] = serde_json::json!([]);

    if let Some(ref kernel) = inline.kernel_capabilities {
        // SAFETY: `kernel_capabilities` was just set to an array literal above.
        let caps = json["kernel_capabilities"]
            .as_array_mut()
            .expect("kernel_capabilities is an array");

        // Add kernel_flags capability (not thread_info)
        if kernel.highest_priority.is_some()
            || kernel.lowest_priority.is_some()
            || kernel.max_core_number.is_some()
            || kernel.min_core_number.is_some()
        {
            // Parser expects nested value object with specific field names
            let mut value = serde_json::Map::new();

            if let Some(highest) = kernel.highest_priority {
                value.insert(
                    "highest_thread_priority".to_string(),
                    serde_json::json!(highest),
                );
            }
            if let Some(lowest) = kernel.lowest_priority {
                value.insert(
                    "lowest_thread_priority".to_string(),
                    serde_json::json!(lowest),
                );
            }
            if let Some(max_core) = kernel.max_core_number {
                value.insert("highest_cpu_id".to_string(), serde_json::json!(max_core));
            }
            if let Some(min_core) = kernel.min_core_number {
                value.insert("lowest_cpu_id".to_string(), serde_json::json!(min_core));
            }

            caps.push(serde_json::json!({
                "type": "kernel_flags",
                "value": value
            }));
        }

        // Add syscalls capability (not syscall_mask)
        // Parser expects object mapping syscall names to hex ID strings
        if !kernel.enable_system_calls.is_empty() {
            let mut syscall_map = serde_json::Map::new();

            for syscall_name in &kernel.enable_system_calls {
                // Map syscall name to actual kernel syscall ID
                let Some(id) = syscall_name_to_id(syscall_name) else {
                    return Err(format!(
                        "Unknown syscall '{}' in inline NPDM metadata. \
                         Valid syscall names must match the Switch kernel ABI (e.g., 'SetHeapSize'). \
                         Check for typos or consult the syscall reference.",
                        syscall_name
                    ));
                };
                syscall_map.insert(syscall_name.clone(), serde_json::json!(id));
            }

            caps.push(serde_json::json!({
                "type": "syscalls",
                "value": syscall_map
            }));
        }

        // Add kernel_version capability if present
        if let Some(ref version) = kernel.kernel_version {
            // Convert dotted version (e.g., "3.0") to hex format if needed
            let version_value = if version.contains('.') {
                kernel_version_to_hex(version).ok_or_else(|| {
                    format!(
                        "Invalid kernel_version format '{}' - expected 'major.minor' where minor < 16",
                        version
                    )
                })?
            } else {
                // Already in hex format, use as-is
                version.clone()
            };

            caps.push(serde_json::json!({
                "type": "min_kernel_version",
                "value": version_value
            }));
        }
    }

    Ok(json)
}

/// Convert inline NPDM from TOML format to JSON format expected by parse_npdm_json
/// Strip `0x` prefix from hex strings for parser compatibility
///
/// The parser uses `u64::from_str_radix(..., 16)` which does not accept `0x` prefixes.
/// This helper normalizes hex strings from TOML to bare hex digits.
fn strip_hex_prefix(s: &str) -> &str {
    s.strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s)
}

/// Map syscall name to actual Nintendo Switch kernel syscall ID
///
/// Returns the kernel syscall ID as hex string, or None if the syscall name is unknown.
/// Uses the standard Nintendo Switch kernel syscall numbering.
fn syscall_name_to_id(name: &str) -> Option<&'static str> {
    match name {
        "SetHeapSize" => Some("1"),
        "SetMemoryPermission" => Some("2"),
        "SetMemoryAttribute" => Some("3"),
        "MapMemory" => Some("4"),
        "UnmapMemory" => Some("5"),
        "QueryMemory" => Some("6"),
        "ExitProcess" => Some("7"),
        "CreateThread" => Some("8"),
        "StartThread" => Some("9"),
        "ExitThread" => Some("a"),
        "SleepThread" => Some("b"),
        "GetThreadPriority" => Some("c"),
        "SetThreadPriority" => Some("d"),
        "GetThreadCoreMask" => Some("e"),
        "SetThreadCoreMask" => Some("f"),
        "GetCurrentProcessorNumber" => Some("10"),
        "SignalEvent" => Some("11"),
        "ClearEvent" => Some("12"),
        "MapSharedMemory" => Some("13"),
        "UnmapSharedMemory" => Some("14"),
        "CreateTransferMemory" => Some("15"),
        "CloseHandle" => Some("16"),
        "ResetSignal" => Some("17"),
        "WaitSynchronization" => Some("18"),
        "CancelSynchronization" => Some("19"),
        "ArbitrateLock" => Some("1a"),
        "ArbitrateUnlock" => Some("1b"),
        "WaitProcessWideKeyAtomic" => Some("1c"),
        "SignalProcessWideKey" => Some("1d"),
        "GetSystemTick" => Some("1e"),
        "ConnectToNamedPort" => Some("1f"),
        "SendSyncRequestLight" => Some("20"),
        "SendSyncRequest" => Some("21"),
        "SendSyncRequestWithUserBuffer" => Some("22"),
        "SendAsyncRequestWithUserBuffer" => Some("23"),
        "GetProcessId" => Some("24"),
        "GetThreadId" => Some("25"),
        "Break" => Some("26"),
        "OutputDebugString" => Some("27"),
        "ReturnFromException" => Some("28"),
        "GetInfo" => Some("29"),
        "FlushEntireDataCache" => Some("2a"),
        "FlushDataCache" => Some("2b"),
        "MapPhysicalMemory" => Some("2c"),
        "UnmapPhysicalMemory" => Some("2d"),
        "GetDebugFutureThreadInfo" => Some("2e"),
        "GetLastThreadInfo" => Some("2f"),
        "GetResourceLimitLimitValue" => Some("30"),
        "GetResourceLimitCurrentValue" => Some("31"),
        "SetThreadActivity" => Some("32"),
        "GetThreadContext3" => Some("33"),
        "WaitForAddress" => Some("34"),
        "SignalToAddress" => Some("35"),
        "SynchronizePreemptionState" => Some("36"),
        "GetResourceLimitPeakValue" => Some("37"),
        "CreateIoPool" => Some("39"),
        "CreateIoRegion" => Some("3a"),
        "KernelDebug" => Some("3c"),
        "ChangeKernelTraceState" => Some("3d"),
        "CreateSession" => Some("40"),
        "AcceptSession" => Some("41"),
        "ReplyAndReceiveLight" => Some("42"),
        "ReplyAndReceive" => Some("43"),
        "ReplyAndReceiveWithUserBuffer" => Some("44"),
        "CreateEvent" => Some("45"),
        "MapIoRegion" => Some("46"),
        "UnmapIoRegion" => Some("47"),
        "MapPhysicalMemoryUnsafe" => Some("48"),
        "UnmapPhysicalMemoryUnsafe" => Some("49"),
        "SetUnsafeLimit" => Some("4a"),
        "CreateCodeMemory" => Some("4b"),
        "ControlCodeMemory" => Some("4c"),
        "SleepSystem" => Some("4d"),
        "ReadWriteRegister" => Some("4e"),
        "SetProcessActivity" => Some("4f"),
        "CreateSharedMemory" => Some("50"),
        "MapTransferMemory" => Some("51"),
        "UnmapTransferMemory" => Some("52"),
        "CreateInterruptEvent" => Some("53"),
        "QueryPhysicalAddress" => Some("54"),
        "QueryIoMapping" => Some("55"),
        "CreateDeviceAddressSpace" => Some("56"),
        "AttachDeviceAddressSpace" => Some("57"),
        "DetachDeviceAddressSpace" => Some("58"),
        "MapDeviceAddressSpaceByForce" => Some("59"),
        "MapDeviceAddressSpaceAligned" => Some("5a"),
        "MapDeviceAddressSpace" => Some("5b"),
        "UnmapDeviceAddressSpace" => Some("5c"),
        "InvalidateProcessDataCache" => Some("5d"),
        "StoreProcessDataCache" => Some("5e"),
        "FlushProcessDataCache" => Some("5f"),
        "DebugActiveProcess" => Some("60"),
        "BreakDebugProcess" => Some("61"),
        "TerminateDebugProcess" => Some("62"),
        "GetDebugEvent" => Some("63"),
        "ContinueDebugEvent" => Some("64"),
        "GetProcessList" => Some("65"),
        "GetThreadList" => Some("66"),
        "GetDebugThreadContext" => Some("67"),
        "SetDebugThreadContext" => Some("68"),
        "QueryDebugProcessMemory" => Some("69"),
        "ReadDebugProcessMemory" => Some("6a"),
        "WriteDebugProcessMemory" => Some("6b"),
        "SetHardwareBreakPoint" => Some("6c"),
        "GetDebugThreadParam" => Some("6d"),
        "GetSystemInfo" => Some("6f"),
        "CreatePort" => Some("70"),
        "ManageNamedPort" => Some("71"),
        "ConnectToPort" => Some("72"),
        "SetProcessMemoryPermission" => Some("73"),
        "MapProcessMemory" => Some("74"),
        "UnmapProcessMemory" => Some("75"),
        "QueryProcessMemory" => Some("76"),
        "MapProcessCodeMemory" => Some("77"),
        "UnmapProcessCodeMemory" => Some("78"),
        "CreateProcess" => Some("79"),
        "StartProcess" => Some("7a"),
        "TerminateProcess" => Some("7b"),
        "GetProcessInfo" => Some("7c"),
        "CreateResourceLimit" => Some("7d"),
        "SetResourceLimitLimitValue" => Some("7e"),
        "CallSecureMonitor" => Some("7f"),
        "MapInsecurePhysicalMemory" => Some("90"),
        "UnmapInsecurePhysicalMemory" => Some("91"),
        _ => None,
    }
}

/// Convert dotted kernel version to hex format
///
/// Examples:
/// - "3.0" -> "30"
/// - "5.1" -> "51"
///
/// Returns `None` if the version string is malformed or if minor >= 16 (which would overflow into major bits).
fn kernel_version_to_hex(version: &str) -> Option<String> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 2 {
        return None;
    }

    let major: u32 = parts[0].parse().ok()?;
    let minor: u32 = parts[1].parse().ok()?;

    // Validate minor < 16 (minor field is 4 bits)
    if minor >= 16 {
        return None;
    }

    // Format: (major << 4) | minor in hex
    let version_value = (major << 4) | minor;
    Some(format!("{:x}", version_value))
}

#[cfg(test)]
mod tests {
    use super::Error;
    use crate::ui::{CliError as _, EXIT_FAILURE};

    #[test]
    fn it_propagates_cargo_build_exit_code() {
        //* Given
        let err = Error::CargoBuildFailed { code: 101 };

        //* When
        let code = err.exit_code();

        //* Then
        assert_eq!(code, 101);
    }

    #[test]
    fn it_defaults_to_failure_exit_code_for_other_errors() {
        //* Given
        let err = Error::MissingNpdm;

        //* When
        let code = err.exit_code();

        //* Then
        assert_eq!(code, EXIT_FAILURE);
    }
}
