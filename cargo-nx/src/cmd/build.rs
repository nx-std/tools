use std::{
    io::BufReader,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use cargo_metadata::{Artifact, Message, MetadataCommand, Package};
use nx_object::{
    elf::ElfSegments,
    read::SetLanguage,
    write::{NacpBuilder, Pfs0Builder, RomFsBuilder, npdm::NpdmBuilder},
};

use crate::cmd::tool::npdmtool::{parse_npdm_json, parse_npdm_json_value};

/// The default target triple to use when building.
const DEFAULT_TARGET_TRIPLE: &str = "aarch64-nintendo-switch-freestanding";

/// The default icon to use when building an NRO.
const DEFAULT_NRO_ICON: &[u8] = include_bytes!("../../default/nro/default_icon.jpg");

/// Handle the `build` subcommand.
pub fn handle_subcommand(args: Args) {
    let metadata = MetadataCommand::new()
        .manifest_path("./Cargo.toml")
        .no_deps()
        .exec()
        .unwrap();

    let rust_target_path = match std::env::var("RUST_TARGET_PATH") {
        Ok(s) => PathBuf::from(s),
        Err(_) => metadata.workspace_root.into_std_path_buf(),
    };

    let target = args.target.as_deref().unwrap_or(DEFAULT_TARGET_TRIPLE);
    if args.verbose {
        println!("Target triple: {}", target);
    }

    let build_target_path = rust_target_path.to_str().unwrap();
    if args.verbose {
        println!("Build target path: {}", build_target_path);
    }

    let mut build_args: Vec<String> = vec![
        String::from("build"),
        format!("--target={}", target),
        String::from("--message-format=json-diagnostic-rendered-ansi"),
    ];
    if args.release {
        build_args.push(String::from("--release"));
    }

    let build_crates: Vec<Package> = match args.package {
        Some(target_package) => {
            vec![
                metadata
                    .packages
                    .iter()
                    .find(|needle| needle.name == target_package)
                    .unwrap_or_else(|| panic!("Failed to find package {target_package}"))
                    .clone(),
            ]
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
            panic!("Error: multiple target formats are not yet supported...");
        } else if is_nsp {
            println!("Building and generating NSP...");
        } else if is_nro {
            println!("Building and generating NRO...");
        } else {
            println!("Building...");
        }

        let mut command = Command::new("cargo")
            .args(&build_args)
            .stdout(Stdio::piped())
            .env("RUST_TARGET_PATH", build_target_path)
            .spawn()
            .unwrap();

        let reader = BufReader::new(command.stdout.take().unwrap());
        for message in Message::parse_stream(reader) {
            match message {
                Ok(Message::CompilerArtifact(ref artifact)) => {
                    if artifact.target.kind.contains(&"bin".into())
                        || artifact.target.kind.contains(&"cdylib".into())
                    {
                        let package: &Package = match metadata
                            .packages
                            .iter()
                            .find(|v| v.id == artifact.package_id)
                        {
                            Some(v) => v,
                            None => continue,
                        };

                        let root = package.manifest_path.parent().unwrap();

                        if is_nsp {
                            let nsp_metadata: NspMetadata = serde_json::from_value(
                                metadata_v.pointer("/nx/nsp").cloned().unwrap(),
                            )
                            .unwrap_or_default();
                            handle_nsp_format(root.as_std_path(), artifact, nsp_metadata);
                        } else if is_nro {
                            let nro_metadata: NroMetadata = serde_json::from_value(
                                metadata_v.pointer("/nx/nro").cloned().unwrap(),
                            )
                            .unwrap_or_default();
                            handle_nro_format(root.as_std_path(), artifact, nro_metadata);
                        }
                    }
                }
                Ok(Message::CompilerMessage(msg)) => {
                    if let Some(msg) = msg.message.rendered {
                        println!("{}", msg);
                    } else {
                        println!("{:?}", msg);
                    }
                }
                Ok(_) => (),
                Err(err) => {
                    panic!("{:?}", err);
                }
            }
        }

        let status = command.wait().unwrap();
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
    }
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

/// Per-language NACP entries (matches linkle's structure for compatibility)
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

fn handle_nro_format(root: &Path, artifact: &Artifact, metadata: NroMetadata) {
    let elf = artifact.filenames[0].clone();
    let nro = get_output_elf_path_as(
        artifact,
        if metadata.overlay == Some(true) {
            "ovl"
        } else {
            "nro"
        },
    );

    // Read ELF file
    let elf_data = std::fs::read(elf.as_std_path()).expect("Failed to read ELF file");

    // Parse ELF segments
    let segments = ElfSegments::parse(&elf_data).expect("Failed to parse ELF segments");

    // Build NRO using nx-object
    let mut nro_builder = segments.into_nro_builder();

    // Add RomFS if specified
    if let Some(romfs_dir) = metadata.romfs.as_ref() {
        let romfs_path = root.join(romfs_dir);
        let romfs_builder = RomFsBuilder::from_directory(&romfs_path)
            .expect("Failed to build RomFS from directory");
        let romfs_data = romfs_builder.build().expect("Failed to build RomFS");
        nro_builder = nro_builder.asset_romfs(romfs_data);
    }

    // Add icon (either user-provided or default)
    let icon_data = if let Some(icon_file) = metadata.icon.as_ref() {
        let icon_path = root.join(icon_file);
        std::fs::read(icon_path).expect("Failed to read icon file")
    } else {
        DEFAULT_NRO_ICON.to_vec()
    };
    nro_builder = nro_builder.asset_icon(icon_data);

    // Add NACP if specified
    if let Some(nacp_metadata) = metadata.nacp {
        // Convert NacpMetadata to NacpBuilder and serialize via nx-object
        let nacp_data =
            build_nacp_from_metadata(&nacp_metadata).expect("Failed to build NACP from metadata");
        nro_builder = nro_builder.asset_nacp(nacp_data);
    }

    // Build NRO
    let nro_data = nro_builder.build().expect("Failed to build NRO");

    // Write NRO file
    std::fs::write(nro.as_path(), nro_data).expect("Failed to write NRO file");

    println!("Built {}", nro.to_string_lossy());
}

fn handle_nsp_format(root: &Path, artifact: &Artifact, metadata: NspMetadata) {
    let elf = artifact.filenames[0].clone();

    let output_path = elf.parent().unwrap();
    let exefs_dir = output_path.join("exefs");
    let _ = std::fs::remove_dir_all(exefs_dir.clone());
    std::fs::create_dir(exefs_dir.clone()).unwrap();

    let main_npdm = exefs_dir.join("main.npdm");
    let main_exe = exefs_dir.join("main");

    let exefs_nsp = get_output_elf_path_as(artifact, "nsp");

    // Parse NPDM (either from inline TOML or external JSON file)
    let (npdm_metadata, aci, acid) = if let Some(inline_npdm) = metadata.npdm {
        // Convert inline NPDM to JSON format and parse in-memory (no temp file)
        let json_value = convert_inline_npdm_to_json(&inline_npdm)
            .expect("Failed to convert inline NPDM to JSON");
        parse_npdm_json_value(&json_value).expect("Failed to parse inline NPDM")
    } else if let Some(npdm_json) = metadata.npdm_json {
        let npdm_json_path = root.join(npdm_json);
        parse_npdm_json(&npdm_json_path).expect("Failed to parse NPDM JSON")
    } else {
        panic!("No npdm or npdm_json specified in package.metadata.nx.nsp")
    };

    let npdm_bytes = NpdmBuilder::new(npdm_metadata)
        .with_aci(aci)
        .with_acid(acid)
        .build();

    std::fs::write(&main_npdm, npdm_bytes).expect("Failed to write NPDM file");

    // Read ELF file and build NSO using nx-object
    let elf_data = std::fs::read(elf.as_std_path()).expect("Failed to read ELF file");
    let segments = ElfSegments::parse(&elf_data).expect("Failed to parse ELF segments");
    let nso_data = segments
        .into_nso_builder()
        .build()
        .expect("Failed to build NSO");
    std::fs::write(&main_exe, nso_data).expect("Failed to write NSO file");

    // Build NSP (PFS0 archive) from exefs directory using nx-object
    let nsp_data = Pfs0Builder::from_directory(exefs_dir.as_str())
        .expect("Failed to create PFS0 builder from exefs directory")
        .build()
        .expect("Failed to build PFS0");
    std::fs::write(&exefs_nsp, nsp_data).expect("Failed to write NSP file");

    println!("Built {}", exefs_nsp.to_string_lossy());
}

/// Convert NACP metadata from Cargo.toml to NACP bytes using nx-object
///
/// This function replicates linkle's NACP serialization behavior:
/// - Uses global name/author/version as defaults for all languages
/// - Per-language entries override the global defaults
/// - Parses title_id and dlc_base_title_id from hex strings
/// - Applies default values matching linkle's behavior
fn build_nacp_from_metadata(metadata: &NacpMetadata) -> Result<Vec<u8>, String> {
    // Use defaults matching linkle's behavior
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
            // Set per-language entries (matching linkle's order and fallback logic)
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

    // Parse title_id if provided (matching linkle's behavior)
    if let Some(ref title_id_str) = metadata.title_id {
        let title_id = u64::from_str_radix(title_id_str, 16)
            .map_err(|err| format!("Invalid title_id '{}': {}", title_id_str, err))?;
        builder = builder.application_id(title_id);
    }

    // Note: dlc_base_title_id is not supported by NacpBuilder yet
    // linkle sets add_on_content_base_id automatically to title_id + 0x1000
    // which NacpBuilder already does, so we don't need to handle it separately

    // Build NACP bytes
    builder
        .build()
        .map_err(|err| format!("Failed to build NACP: {}", err))
}

fn get_output_elf_path_as(artifact: &Artifact, extension: &str) -> PathBuf {
    let mut elf = artifact.filenames[0].clone();
    assert!(elf.set_extension(extension));
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
        let caps = json["kernel_capabilities"].as_array_mut().unwrap();

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
/// Based on vendor/nx-std-mono/subprojects/nx-svc/include/nx_svc.h syscall numbers.
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
