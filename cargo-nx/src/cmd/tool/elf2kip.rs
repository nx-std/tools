use std::{
    collections::HashMap,
    fs,
    io::{self, BufWriter},
    num::ParseIntError,
    path::PathBuf,
};

use nx_object::{
    elf::{self, ElfSegments},
    write::{
        kip,
        npdm::{KernelCapability, MapRegionDescriptor},
    },
};
use serde::Deserialize;

pub fn handle_subcommand(args: Args) -> Result<(), Error> {
    // Load and parse ELF file
    let elf_data = fs::read(&args.elf_file).map_err(|err| Error::ReadElf {
        path: args.elf_file.clone(),
        source: err,
    })?;

    let segments = ElfSegments::parse(&elf_data).map_err(|err| Error::ParseElf {
        path: args.elf_file.clone(),
        source: err,
    })?;

    // Parse JSON descriptor
    let json_data = fs::read_to_string(&args.json_file).map_err(|err| Error::ReadJson {
        path: args.json_file.clone(),
        source: err,
    })?;

    // Parse JSON into raw value for kernel capabilities extraction
    let json_value: serde_json::Value =
        serde_json::from_str(&json_data).map_err(|err| Error::ParseJsonValue {
            path: args.json_file.clone(),
            source: err,
        })?;

    let descriptor: KipDescriptor =
        serde_json::from_str(&json_data).map_err(|err| Error::DeserializeDescriptor {
            path: args.json_file.clone(),
            source: err,
        })?;

    // Parse kernel capabilities manually to handle hex strings and address-to-page conversion
    let kernel_capabilities = parse_kernel_capabilities(&json_value)?;

    // Encode kernel capabilities
    let capabilities: Vec<u32> = kernel_capabilities
        .iter()
        .flat_map(|cap| cap.encode())
        .collect();

    // Build KIP1 using nx-object builder
    let mut builder = nx_object::write::Kip1Builder::new()
        .name(descriptor.name)
        .title_id(u64::from(descriptor.title_id))
        .process_category(u64::from(descriptor.process_category) as u32)
        .main_thread_priority(descriptor.main_thread_priority)
        .default_cpu_id(descriptor.default_cpu_id)
        .rodata_attributes(u64::from(descriptor.main_thread_stack_size) as u32)
        .kernel_capabilities(capabilities);

    // Compute flags: raw override takes precedence, otherwise use boolean fields.
    // use_secure_memory controls bit 5, immortal controls bit 6.
    if let Some(flags) = descriptor.flags {
        // Raw flags override: use as-is
        builder = builder.flags(flags);
    } else {
        // Compute flags from boolean fields
        // Start with base flags 0x3F:
        // bits 0-5 set (compression + Is64Bit + IsAddrSpace32Bit + UseSystemPoolPartition)
        let mut flags = 0x3F;

        // Apply boolean modifiers
        if descriptor.use_secure_memory {
            flags |= 0x20; // Set bit 5
        } else {
            flags &= !0x20; // Clear bit 5
        }

        if descriptor.immortal {
            flags |= 0x40; // Set bit 6
        } else {
            flags &= !0x40; // Clear bit 6
        }

        builder = builder.flags(flags);
    }

    // Extract segments and build KIP1
    let text = segments.text();
    let text_vaddr = segments.text_vaddr();
    let rodata = segments.rodata();
    let rodata_vaddr = segments.rodata_vaddr();
    let data = segments.data();
    let data_vaddr = segments.data_vaddr();

    builder = builder
        .text(text.to_vec())
        .text_vaddr(text_vaddr as u32)
        .rodata(rodata.to_vec())
        .rodata_vaddr(rodata_vaddr as u32)
        .data(data.to_vec())
        .data_vaddr(data_vaddr as u32);

    // Add BSS if present
    let bss_size = segments.bss_size();
    if bss_size > 0 {
        // BSS vaddr follows data segment, page-aligned to 0x1000
        // The kernel maps segments at page granularity, so we must align the data
        // segment size before computing the BSS base address to prevent overlap
        let bss_vaddr = data_vaddr + ((data.len() as u64 + 0xFFF) & !0xFFF);
        builder = builder
            .bss_vaddr(bss_vaddr as u32)
            .bss_size(bss_size as u32);
    }

    // Build KIP1 binary
    let kip_data = builder.build().map_err(Error::BuildKip)?;

    // Write output file
    let output_file = fs::File::create(&args.kip_file).map_err(|err| Error::CreateOutput {
        path: args.kip_file.clone(),
        source: err,
    })?;

    let mut writer = BufWriter::new(output_file);
    std::io::Write::write_all(&mut writer, &kip_data).map_err(|err| Error::WriteOutput {
        path: args.kip_file.clone(),
        source: err,
    })?;

    Ok(())
}

#[derive(clap::Args)]
pub struct Args {
    /// Path to the input ELF file
    pub elf_file: PathBuf,

    /// Path to the JSON descriptor file
    pub json_file: PathBuf,

    /// Path to the output KIP file
    pub kip_file: PathBuf,
}

/// Errors from the `elf2kip` subcommand
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to read the input ELF file from disk
    #[error("Failed to read ELF file '{}'", path.display())]
    ReadElf { path: PathBuf, source: io::Error },

    /// Failed to parse ELF segments from the input file
    #[error("Failed to parse ELF file '{}'", path.display())]
    ParseElf {
        path: PathBuf,
        source: elf::ParseError,
    },

    /// Failed to read the JSON descriptor file from disk
    #[error("Failed to read JSON file '{}'", path.display())]
    ReadJson { path: PathBuf, source: io::Error },

    /// Failed to parse JSON from the descriptor file into a Value
    #[error("Failed to parse JSON file '{}'", path.display())]
    ParseJsonValue {
        path: PathBuf,
        source: serde_json::Error,
    },

    /// Failed to deserialize the KIP descriptor from JSON
    #[error("Failed to deserialize KIP descriptor from '{}'", path.display())]
    DeserializeDescriptor {
        path: PathBuf,
        source: serde_json::Error,
    },

    /// Invalid or missing argument in the JSON descriptor
    #[error("{0}")]
    InvalidArgs(String),

    /// Failed to parse a hexadecimal value
    #[error("Failed to parse {field} '{value}' as hexadecimal")]
    ParseHex {
        field: String,
        value: String,
        source: ParseIntError,
    },

    /// Failed to build the KIP binary
    #[error("Failed to build KIP")]
    BuildKip(#[source] kip::BuildError),

    /// Failed to create the output file
    #[error("Failed to create KIP output file '{}'", path.display())]
    CreateOutput { path: PathBuf, source: io::Error },

    /// Failed to write the KIP data to the output file
    #[error("Failed to write KIP file '{}'", path.display())]
    WriteOutput { path: PathBuf, source: io::Error },
}

/// HexOrNum helper for deserializing hex strings or numeric values.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(untagged)]
enum HexOrNum {
    Num(u64),
    Hex(HexString),
}

#[derive(Debug, Clone, Copy)]
struct HexString(u64);

impl<'de> Deserialize<'de> for HexString {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let s = s.trim_start_matches("0x").trim_start_matches("0X");
        u64::from_str_radix(s, 16)
            .map(HexString)
            .map_err(serde::de::Error::custom)
    }
}

impl From<HexOrNum> for u64 {
    fn from(value: HexOrNum) -> Self {
        match value {
            HexOrNum::Num(n) => n,
            HexOrNum::Hex(HexString(n)) => n,
        }
    }
}

/// JSON descriptor for KIP1 metadata.
///
/// This struct is used for initial JSON parsing. The `kernel_capabilities` field
/// is parsed separately using manual parsing to handle hex strings and address-to-page
/// conversion.
#[derive(Debug, Deserialize)]
struct KipDescriptor {
    name: String,
    #[serde(alias = "program_id")]
    title_id: HexOrNum,
    main_thread_stack_size: HexOrNum,
    main_thread_priority: u8,
    default_cpu_id: u8,
    #[serde(alias = "version", default = "default_process_category")]
    process_category: HexOrNum,
    #[serde(default)]
    flags: Option<u8>,
    /// Optional boolean to control bit 5 (UseSystemPoolPartition) of the flags byte.
    /// Defaults to true (bit set) if not provided.
    /// Ignored if raw `flags` field is present.
    #[serde(default = "default_use_secure_memory")]
    use_secure_memory: bool,
    /// Optional boolean to control bit 6 (Immortal) of the flags byte.
    /// Defaults to true (bit set) if not provided.
    /// Ignored if raw `flags` field is present.
    #[serde(default = "default_immortal")]
    immortal: bool,
}

/// Default process category value.
fn default_process_category() -> HexOrNum {
    HexOrNum::Num(1)
}

/// Default use_secure_memory value.
fn default_use_secure_memory() -> bool {
    true
}

/// Default immortal value.
fn default_immortal() -> bool {
    true
}

/// Parse kernel capabilities from JSON.
///
/// This function manually parses the `kernel_capabilities` array from JSON to handle:
/// - Hex string values (e.g., `"0x1A"`) for numeric fields
/// - Address-to-page conversion for Map and MapPage capabilities
/// - snake_case type discriminators
fn parse_kernel_capabilities(json: &serde_json::Value) -> Result<Vec<KernelCapability>, Error> {
    let capabilities_array = json
        .get("kernel_capabilities")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            Error::InvalidArgs("Missing or invalid 'kernel_capabilities' array in JSON".into())
        })?;

    let mut capabilities = Vec::new();

    for kac in capabilities_array {
        let type_str = kac
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::InvalidArgs("Kernel capability missing 'type' field".into()))?;

        match type_str {
            "kernel_flags" => {
                let value = kac.get("value").ok_or_else(|| {
                    Error::InvalidArgs("kernel_flags capability missing 'value' object".into())
                })?;

                let highest_thread_priority = value
                    .get("highest_thread_priority")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| {
                        Error::InvalidArgs("kernel_flags missing 'highest_thread_priority'".into())
                    })? as u8;

                let lowest_thread_priority = value
                    .get("lowest_thread_priority")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| {
                        Error::InvalidArgs("kernel_flags missing 'lowest_thread_priority'".into())
                    })? as u8;

                let highest_cpu_id = value
                    .get("highest_cpu_id")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| {
                        Error::InvalidArgs("kernel_flags missing 'highest_cpu_id'".into())
                    })? as u8;

                let lowest_cpu_id = value
                    .get("lowest_cpu_id")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| {
                        Error::InvalidArgs("kernel_flags missing 'lowest_cpu_id'".into())
                    })? as u8;

                capabilities.push(KernelCapability::KernelFlags {
                    highest_thread_priority,
                    lowest_thread_priority,
                    highest_cpu_id,
                    lowest_cpu_id,
                });
            }
            "syscalls" => {
                let syscalls_obj =
                    kac.get("value")
                        .and_then(|v| v.as_object())
                        .ok_or_else(|| {
                            Error::InvalidArgs("syscalls capability missing 'value' object".into())
                        })?;

                let mut syscalls = HashMap::new();
                for (name, value) in syscalls_obj {
                    // Parse syscall ID (supports both hex strings and numbers)
                    let syscall_id = if let Some(num) = value.as_u64() {
                        num
                    } else if let Some(hex_str) = value.as_str() {
                        let hex_str = hex_str.trim_start_matches("0x").trim_start_matches("0X");
                        u64::from_str_radix(hex_str, 16).map_err(|err| Error::ParseHex {
                            field: format!("syscall '{name}'"),
                            value: hex_str.into(),
                            source: err,
                        })?
                    } else {
                        return Err(Error::InvalidArgs(
                            "Syscall values must be numbers or hex strings".into(),
                        ));
                    };

                    syscalls.insert(name.clone(), syscall_id);
                }

                capabilities.push(KernelCapability::Syscalls(syscalls));
            }
            "map" => {
                let value = kac
                    .get("value")
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| {
                        Error::InvalidArgs("map capability missing 'value' object".into())
                    })?;

                // Parse address (supports hex strings)
                let address_bytes = parse_hex_or_num(value, "address")
                    .ok_or_else(|| Error::InvalidArgs("map capability missing 'address'".into()))?;

                // Parse size (supports hex strings)
                let size_bytes = parse_hex_or_num(value, "size")
                    .ok_or_else(|| Error::InvalidArgs("map capability missing 'size'".into()))?;

                let is_ro = value
                    .get("is_ro")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let is_io = value
                    .get("is_io")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Convert byte addresses to page numbers (>> 12)
                let address = address_bytes >> 12;
                let size = size_bytes >> 12;

                capabilities.push(KernelCapability::Map {
                    address,
                    size,
                    is_ro,
                    is_io,
                });
            }
            "map_page" => {
                let value = kac.get("value").ok_or_else(|| {
                    Error::InvalidArgs("map_page capability missing 'value' field".into())
                })?;

                // Parse page address (supports hex strings)
                let page_address_bytes = if let Some(num) = value.as_u64() {
                    num
                } else if let Some(hex_str) = value.as_str() {
                    let hex_str = hex_str.trim_start_matches("0x").trim_start_matches("0X");
                    u64::from_str_radix(hex_str, 16).map_err(|err| Error::ParseHex {
                        field: "map_page value".into(),
                        value: hex_str.into(),
                        source: err,
                    })?
                } else {
                    return Err(Error::InvalidArgs(
                        "map_page value must be a number or hex string".into(),
                    ));
                };

                // Convert byte address to page number (>> 12)
                let page = page_address_bytes >> 12;

                capabilities.push(KernelCapability::MapPage(page));
            }
            "map_region" => {
                let value_array = kac.get("value").and_then(|v| v.as_array()).ok_or_else(|| {
                    Error::InvalidArgs("map_region capability missing 'value' array".into())
                })?;

                if value_array.len() > 3 {
                    return Err(Error::InvalidArgs(
                        "map_region value array can have at most 3 region descriptors".into(),
                    ));
                }

                let mut regions = Vec::new();
                for (idx, region_value) in value_array.iter().enumerate() {
                    let region_obj = region_value.as_object().ok_or_else(|| {
                        Error::InvalidArgs(format!("map_region value[{}] must be an object", idx))
                    })?;

                    let region_type = region_obj
                        .get("region_type")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| {
                            Error::InvalidArgs(format!(
                                "map_region value[{}] missing 'region_type'",
                                idx
                            ))
                        })? as u8;

                    let is_ro = region_obj
                        .get("is_ro")
                        .and_then(|v| v.as_bool())
                        .ok_or_else(|| {
                            Error::InvalidArgs(format!("map_region value[{}] missing 'is_ro'", idx))
                        })?;

                    regions.push(MapRegionDescriptor { region_type, is_ro });
                }

                capabilities.push(KernelCapability::MapRegion(regions));
            }
            "irq_pair" => {
                // The "value" field holds an array of exactly 2 IRQ entries
                let value_array = kac.get("value").and_then(|v| v.as_array()).ok_or_else(|| {
                    Error::InvalidArgs("irq_pair capability missing 'value' array".into())
                })?;

                if value_array.len() != 2 {
                    return Err(Error::InvalidArgs(
                        "irq_pair value must be an array with exactly 2 elements".into(),
                    ));
                }

                // JSON null entries are converted to 0x3FF (10-bit "unused" sentinel)
                let irq0 = if value_array[0].is_null() {
                    0x3FF
                } else {
                    value_array[0].as_u64().ok_or_else(|| {
                        Error::InvalidArgs("irq_pair value[0] must be a number or null".into())
                    })? as u16
                };

                let irq1 = if value_array[1].is_null() {
                    0x3FF
                } else {
                    value_array[1].as_u64().ok_or_else(|| {
                        Error::InvalidArgs("irq_pair value[1] must be a number or null".into())
                    })? as u16
                };

                capabilities.push(KernelCapability::IrqPair([irq0, irq1]));
            }
            "application_type" => {
                let app_type = kac.get("value").and_then(|v| v.as_u64()).ok_or_else(|| {
                    Error::InvalidArgs("application_type capability missing numeric 'value'".into())
                })? as u16;

                capabilities.push(KernelCapability::ApplicationType(app_type));
            }
            "min_kernel_version" => {
                let value = kac.get("value").ok_or_else(|| {
                    Error::InvalidArgs("min_kernel_version capability missing 'value'".into())
                })?;

                // Parse version (supports hex strings)
                let version = if let Some(num) = value.as_u64() {
                    num
                } else if let Some(hex_str) = value.as_str() {
                    let hex_str = hex_str.trim_start_matches("0x").trim_start_matches("0X");
                    u64::from_str_radix(hex_str, 16).map_err(|err| Error::ParseHex {
                        field: "min_kernel_version".into(),
                        value: hex_str.into(),
                        source: err,
                    })?
                } else {
                    return Err(Error::InvalidArgs(
                        "min_kernel_version value must be a number or hex string".into(),
                    ));
                };

                capabilities.push(KernelCapability::MinKernelVersion(version));
            }
            "handle_table_size" => {
                let size = kac.get("value").and_then(|v| v.as_u64()).ok_or_else(|| {
                    Error::InvalidArgs(
                        "handle_table_size capability missing numeric 'value'".into(),
                    )
                })? as u16;

                capabilities.push(KernelCapability::HandleTableSize(size));
            }
            "debug_flags" => {
                let value = kac
                    .get("value")
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| {
                        Error::InvalidArgs("debug_flags capability missing 'value' object".into())
                    })?;

                let allow_debug = value
                    .get("allow_debug")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let force_debug_prod = value
                    .get("force_debug_prod")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let force_debug = value
                    .get("force_debug")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                capabilities.push(KernelCapability::DebugFlags {
                    allow_debug,
                    force_debug_prod,
                    force_debug,
                });
            }
            _ => {
                return Err(Error::InvalidArgs(format!(
                    "Unknown kernel capability type: '{}'",
                    type_str
                )));
            }
        }
    }

    Ok(capabilities)
}

/// Parse a JSON field that can be either a u64 number or a hex string.
fn parse_hex_or_num(obj: &serde_json::Map<String, serde_json::Value>, field: &str) -> Option<u64> {
    obj.get(field).and_then(|v| {
        if let Some(num) = v.as_u64() {
            Some(num)
        } else if let Some(hex_str) = v.as_str() {
            let hex_str = hex_str.trim_start_matches("0x").trim_start_matches("0X");
            u64::from_str_radix(hex_str, 16).ok()
        } else {
            None
        }
    })
}
