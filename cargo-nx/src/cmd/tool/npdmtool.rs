use std::{collections::HashMap, io, num::ParseIntError, path::PathBuf};

use nx_object::write::npdm::{
    AciData, AcidData, FilesystemAccess, KernelCapability, MapRegionDescriptor, NpdmBuilder,
    NpdmMetadata, SaveDataOwnerId, ServiceAccess,
};

pub fn handle_subcommand(args: Args) -> Result<(), Error> {
    let (metadata, aci, acid) = parse_npdm_json(&args.json_file)?;

    let npdm_bytes = NpdmBuilder::new(metadata)
        .with_aci(aci)
        .with_acid(acid)
        .build();

    std::fs::write(&args.npdm_file, npdm_bytes).map_err(|err| Error::WriteNpdm {
        path: args.npdm_file.clone(),
        source: err,
    })?;

    Ok(())
}

#[derive(clap::Args)]
pub struct Args {
    /// Path to the input JSON descriptor file
    pub json_file: PathBuf,

    /// Path to the output NPDM file
    pub npdm_file: PathBuf,
}

/// Errors from the `npdmtool` subcommand
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to read the JSON descriptor file from disk
    #[error("Failed to read JSON file '{}'", path.display())]
    ReadJson { path: PathBuf, source: io::Error },

    /// Failed to parse JSON from the descriptor file
    #[error("Failed to parse JSON file '{}'", path.display())]
    ParseJson {
        path: PathBuf,
        source: serde_json::Error,
    },

    /// Invalid or missing argument in the JSON descriptor
    #[error("{0}")]
    InvalidArgs(String),

    /// Failed to parse a hexadecimal value from the JSON descriptor
    #[error("Failed to parse {field} '{value}' as hexadecimal")]
    ParseHex {
        field: String,
        value: String,
        source: ParseIntError,
    },

    /// Failed to write the NPDM output file to disk
    #[error("Failed to write NPDM file '{}'", path.display())]
    WriteNpdm { path: PathBuf, source: io::Error },
}

/// Parse NPDM JSON descriptor from a file path
///
/// Reads JSON file from disk and delegates to `parse_npdm_json_value` for parsing.
pub fn parse_npdm_json(json_path: &PathBuf) -> Result<(NpdmMetadata, AciData, AcidData), Error> {
    let json_content = std::fs::read_to_string(json_path).map_err(|err| Error::ReadJson {
        path: json_path.clone(),
        source: err,
    })?;

    let json: serde_json::Value =
        serde_json::from_str(&json_content).map_err(|err| Error::ParseJson {
            path: json_path.clone(),
            source: err,
        })?;

    parse_npdm_json_value(&json)
}

/// Parse NPDM JSON descriptor from a serde_json::Value
///
/// This is the core parser that accepts pre-parsed JSON. Used for both file-based
/// and in-memory JSON parsing (inline NPDM metadata from TOML).
pub fn parse_npdm_json_value(
    json: &serde_json::Value,
) -> Result<(NpdmMetadata, AciData, AcidData), Error> {
    // Extract required fields
    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::InvalidArgs("Missing required field 'name'".into()))?
        .to_string();

    let program_id = json
        .get("program_id")
        .or_else(|| json.get("title_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            Error::InvalidArgs("Missing required field 'program_id' or 'title_id'".into())
        })?;
    let title_id = parse_hex_u64(program_id).map_err(|err| Error::ParseHex {
        field: "program_id".into(),
        value: program_id.into(),
        source: err,
    })?;

    let main_thread_stack_size_str = json
        .get("main_thread_stack_size")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            Error::InvalidArgs("Missing required field 'main_thread_stack_size'".into())
        })?;
    let main_thread_stack_size_u64 =
        parse_hex_u64(main_thread_stack_size_str).map_err(|err| Error::ParseHex {
            field: "main_thread_stack_size".into(),
            value: main_thread_stack_size_str.into(),
            source: err,
        })?;

    if main_thread_stack_size_u64 > u32::MAX as u64 {
        return Err(Error::InvalidArgs(
            "main_thread_stack_size exceeds u32::MAX".into(),
        ));
    }
    let main_thread_stack_size = main_thread_stack_size_u64 as u32;

    let main_thread_priority_u64 = json
        .get("main_thread_priority")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            Error::InvalidArgs("Missing required field 'main_thread_priority'".into())
        })?;

    if main_thread_priority_u64 > 63 {
        return Err(Error::InvalidArgs(
            "main_thread_priority must be in range 0-63".into(),
        ));
    }
    let main_thread_priority = main_thread_priority_u64 as u8;

    let default_cpu_id_u64 = json
        .get("default_cpu_id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| Error::InvalidArgs("Missing required field 'default_cpu_id'".into()))?;

    if default_cpu_id_u64 > 3 {
        return Err(Error::InvalidArgs(
            "default_cpu_id must be in range 0-3".into(),
        ));
    }
    let default_cpu_id = default_cpu_id_u64 as u8;

    // Version (optional, defaults to 0)
    let version = match json
        .get("version")
        .or_else(|| json.get("process_category"))
        .and_then(|v| v.as_str())
    {
        Some(s) => parse_hex_u64(s)
            .map_err(|err| Error::ParseHex {
                field: "version/process_category".into(),
                value: s.into(),
                source: err,
            })?
            .min(u32::MAX as u64) as u32,
        None => 0,
    };

    // Compute MMU flags
    let address_space_type_u64 = json
        .get("address_space_type")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| Error::InvalidArgs("Missing required field 'address_space_type'".into()))?;

    if address_space_type_u64 > 3 {
        return Err(Error::InvalidArgs(
            "address_space_type must be in range 0-3".into(),
        ));
    }
    let address_space_type = address_space_type_u64 as u8;
    let is_64_bit = json
        .get("is_64_bit")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| Error::InvalidArgs("Missing required field 'is_64_bit'".into()))?;

    let mut flags = ((address_space_type & 3) << 1) | (is_64_bit as u8);

    if let Some(optimize_memory_allocation) = json
        .get("optimize_memory_allocation")
        .and_then(|v| v.as_bool())
    {
        flags |= (optimize_memory_allocation as u8) << 4;
    }

    if let Some(disable_device_address_space_merge) = json
        .get("disable_device_address_space_merge")
        .and_then(|v| v.as_bool())
    {
        flags |= (disable_device_address_space_merge as u8) << 5;
    }

    if let Some(enable_alias_region_extra_size) = json
        .get("enable_alias_region_extra_size")
        .and_then(|v| v.as_bool())
    {
        flags |= (enable_alias_region_extra_size as u8) << 6;
    }

    if let Some(prevent_code_reads) = json.get("prevent_code_reads").and_then(|v| v.as_bool()) {
        flags |= (prevent_code_reads as u8) << 7;
    }

    let metadata = NpdmMetadata {
        name,
        title_id,
        version,
        main_thread_priority,
        default_cpu_id,
        main_thread_stack_size,
        flags,
    };

    // Parse ACI0 data
    let filesystem_access = parse_filesystem_access(json)?;
    let service_access = parse_service_access(json)?;
    let kernel_capabilities = parse_kernel_capabilities(json)?;

    let aci = AciData {
        program_id: title_id,
        filesystem_access: filesystem_access.clone(),
        service_access: service_access.clone(),
        kernel_capabilities: kernel_capabilities.clone(),
    };

    // Parse ACID data
    let is_retail = json
        .get("is_retail")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| Error::InvalidArgs("Missing required field 'is_retail'".into()))?;

    let pool_partition_u64 = json
        .get("pool_partition")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| Error::InvalidArgs("Missing required field 'pool_partition'".into()))?;

    if pool_partition_u64 > 3 {
        return Err(Error::InvalidArgs(
            "pool_partition must be in range 0-3".into(),
        ));
    }
    let pool_partition = pool_partition_u64 as u8;

    let acid_flags = (is_retail as u32) | ((pool_partition as u32 & 3) << 2);

    let program_id_range_min_str = json
        .get("program_id_range_min")
        .or_else(|| json.get("title_id_range_min"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            Error::InvalidArgs(
                "Missing required field 'program_id_range_min' or 'title_id_range_min'".into(),
            )
        })?;
    let program_id_min =
        parse_hex_u64(program_id_range_min_str).map_err(|err| Error::ParseHex {
            field: "program_id_range_min".into(),
            value: program_id_range_min_str.into(),
            source: err,
        })?;

    let program_id_range_max_str = json
        .get("program_id_range_max")
        .or_else(|| json.get("title_id_range_max"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            Error::InvalidArgs(
                "Missing required field 'program_id_range_max' or 'title_id_range_max'".into(),
            )
        })?;
    let program_id_max =
        parse_hex_u64(program_id_range_max_str).map_err(|err| Error::ParseHex {
            field: "program_id_range_max".into(),
            value: program_id_range_max_str.into(),
            source: err,
        })?;

    let acid = AcidData {
        program_id_min,
        program_id_max,
        filesystem_access,
        service_access,
        kernel_capabilities,
        flags: acid_flags,
    };

    Ok((metadata, aci, acid))
}

/// Parse filesystem access from JSON.
fn parse_filesystem_access(json: &serde_json::Value) -> Result<FilesystemAccess, Error> {
    let fsaccess = json
        .get("filesystem_access")
        .ok_or_else(|| Error::InvalidArgs("Missing required field 'filesystem_access'".into()))?;

    let permissions_str = fsaccess
        .get("permissions")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            Error::InvalidArgs("Missing required field 'filesystem_access.permissions'".into())
        })?;
    let permissions = parse_hex_u64(permissions_str).map_err(|err| Error::ParseHex {
        field: "filesystem_access.permissions".into(),
        value: permissions_str.into(),
        source: err,
    })?;

    // Parse content_owner_ids array (optional)
    let mut content_owner_ids = Vec::new();
    if let Some(cois_value) = fsaccess.get("content_owner_ids") {
        let cois_array = cois_value.as_array().ok_or_else(|| {
            Error::InvalidArgs("Field 'content_owner_ids' must be an array if present".into())
        })?;

        for coi in cois_array {
            let id_str = coi.as_str().ok_or_else(|| {
                Error::InvalidArgs("content_owner_ids entries must be hex strings".into())
            })?;
            let id = parse_hex_u64(id_str).map_err(|err| Error::ParseHex {
                field: "content_owner_id".into(),
                value: id_str.into(),
                source: err,
            })?;
            content_owner_ids.push(id);
        }
    }

    // Parse save_data_owner_ids array (optional)
    let mut save_data_owner_ids = Vec::new();
    if let Some(sdois_value) = fsaccess.get("save_data_owner_ids") {
        let sdois_array = sdois_value.as_array().ok_or_else(|| {
            Error::InvalidArgs("Field 'save_data_owner_ids' must be an array if present".into())
        })?;

        for sdoi in sdois_array {
            let sdoi_obj = sdoi.as_object().ok_or_else(|| {
                Error::InvalidArgs("save_data_owner_ids entries must be objects".into())
            })?;

            let accessibility_u64 = sdoi_obj
                .get("accessibility")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| {
                    Error::InvalidArgs(
                        "save_data_owner_ids entry missing 'accessibility' u64 field".into(),
                    )
                })?;

            if accessibility_u64 > 3 {
                return Err(Error::InvalidArgs(
                    "save_data_owner_ids[].accessibility must be in range 0-3".into(),
                ));
            }
            let accessibility = accessibility_u64 as u8;

            let id_str = sdoi_obj.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                Error::InvalidArgs("save_data_owner_ids entry missing 'id' hex string field".into())
            })?;
            let id = parse_hex_u64(id_str).map_err(|err| Error::ParseHex {
                field: "save_data_owner_id".into(),
                value: id_str.into(),
                source: err,
            })?;

            save_data_owner_ids.push(SaveDataOwnerId { accessibility, id });
        }
    }

    Ok(FilesystemAccess {
        permissions,
        content_owner_ids,
        save_data_owner_ids,
        content_owner_id_min: 0,
        content_owner_id_max: 0,
        save_data_owner_id_min: 0,
        save_data_owner_id_max: 0,
    })
}

/// Parse service access from JSON.
fn parse_service_access(json: &serde_json::Value) -> Result<ServiceAccess, Error> {
    let mut services = Vec::new();

    // Parse service_host array first
    // These are services this program provides (is_host = true)
    if let Some(service_host_value) = json.get("service_host") {
        let service_host_array = service_host_value.as_array().ok_or_else(|| {
            Error::InvalidArgs("Field 'service_host' must be an array if present".into())
        })?;

        for service in service_host_array {
            let service_name = service
                .as_str()
                .ok_or_else(|| Error::InvalidArgs("service_host entries must be strings".into()))?;

            services.push((service_name.to_string(), true));
        }
    }

    // Parse service_access array (optional)
    // These are services this program accesses as a client (is_host = false)
    if let Some(services_value) = json.get("service_access") {
        let services_list = services_value.as_array().ok_or_else(|| {
            Error::InvalidArgs("Field 'service_access' must be an array if present".into())
        })?;

        for service in services_list {
            let service_name = service.as_str().ok_or_else(|| {
                Error::InvalidArgs("service_access entries must be strings".into())
            })?;

            services.push((service_name.to_string(), false));
        }
    }

    Ok(ServiceAccess { services })
}

/// Parse kernel capabilities from JSON.
fn parse_kernel_capabilities(json: &serde_json::Value) -> Result<Vec<KernelCapability>, Error> {
    // kernel_capabilities is a required field
    let kac_value = json
        .get("kernel_capabilities")
        .ok_or_else(|| Error::InvalidArgs("Missing required field 'kernel_capabilities'".into()))?;

    let kac_array = kac_value
        .as_array()
        .ok_or_else(|| Error::InvalidArgs("Field 'kernel_capabilities' must be an array".into()))?;

    let mut capabilities = Vec::new();

    for kac in kac_array {
        let kac_type = kac
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::InvalidArgs("Kernel capability missing 'type' field".into()))?;

        match kac_type {
            "kernel_flags" => {
                // The "value" object holds the per-flag priority and CPU-id fields
                let value = kac
                    .get("value")
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| {
                        Error::InvalidArgs("kernel_flags capability missing 'value' object".into())
                    })?;

                let highest_thread_priority_u64 = value
                    .get("highest_thread_priority")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| {
                        Error::InvalidArgs(
                            "kernel_flags.value missing 'highest_thread_priority'".into(),
                        )
                    })?;

                if highest_thread_priority_u64 > 63 {
                    return Err(Error::InvalidArgs(
                        "kernel_flags.highest_thread_priority must be in range 0-63".into(),
                    ));
                }
                let highest_thread_priority = highest_thread_priority_u64 as u8;

                let lowest_thread_priority_u64 = value
                    .get("lowest_thread_priority")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| {
                        Error::InvalidArgs(
                            "kernel_flags.value missing 'lowest_thread_priority'".into(),
                        )
                    })?;

                if lowest_thread_priority_u64 > 63 {
                    return Err(Error::InvalidArgs(
                        "kernel_flags.lowest_thread_priority must be in range 0-63".into(),
                    ));
                }
                let lowest_thread_priority = lowest_thread_priority_u64 as u8;

                let highest_cpu_id_u64 = value
                    .get("highest_cpu_id")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| {
                        Error::InvalidArgs("kernel_flags.value missing 'highest_cpu_id'".into())
                    })?;

                if highest_cpu_id_u64 > 255 {
                    return Err(Error::InvalidArgs(
                        "kernel_flags.highest_cpu_id must be in range 0-255".into(),
                    ));
                }
                let highest_cpu_id = highest_cpu_id_u64 as u8;

                let lowest_cpu_id_u64 = value
                    .get("lowest_cpu_id")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| {
                        Error::InvalidArgs("kernel_flags.value missing 'lowest_cpu_id'".into())
                    })?;

                if lowest_cpu_id_u64 > 255 {
                    return Err(Error::InvalidArgs(
                        "kernel_flags.lowest_cpu_id must be in range 0-255".into(),
                    ));
                }
                let lowest_cpu_id = lowest_cpu_id_u64 as u8;

                capabilities.push(KernelCapability::KernelFlags {
                    highest_thread_priority,
                    lowest_thread_priority,
                    highest_cpu_id,
                    lowest_cpu_id,
                });
            }
            "syscalls" => {
                // The "value" object maps syscall names to their IDs
                let syscalls_obj =
                    kac.get("value")
                        .and_then(|v| v.as_object())
                        .ok_or_else(|| {
                            Error::InvalidArgs("syscalls capability missing 'value' object".into())
                        })?;

                let mut syscalls = HashMap::new();
                for (name, value) in syscalls_obj {
                    let syscall_id = if let Some(id_str) = value.as_str() {
                        parse_hex_u64(id_str).map_err(|err| Error::ParseHex {
                            field: format!("syscall '{name}'"),
                            value: id_str.into(),
                            source: err,
                        })?
                    } else if let Some(id_num) = value.as_u64() {
                        id_num
                    } else {
                        return Err(Error::InvalidArgs(format!(
                            "Invalid syscall value for '{}'",
                            name
                        )));
                    };

                    syscalls.insert(name.clone(), syscall_id);
                }

                capabilities.push(KernelCapability::Syscalls(syscalls));
            }
            "map" => {
                // The "value" object holds the address, size, and permission fields
                let value = kac
                    .get("value")
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| {
                        Error::InvalidArgs("map capability missing 'value' object".into())
                    })?;

                let address_str = value
                    .get("address")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::InvalidArgs("map.value missing 'address'".into()))?;
                let address_bytes = parse_hex_u64(address_str).map_err(|err| Error::ParseHex {
                    field: "map address".into(),
                    value: address_str.into(),
                    source: err,
                })?;

                let size_str = value
                    .get("size")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::InvalidArgs("map.value missing 'size'".into()))?;
                let size_bytes = parse_hex_u64(size_str).map_err(|err| Error::ParseHex {
                    field: "map size".into(),
                    value: size_str.into(),
                    source: err,
                })?;

                // Both is_ro and is_io are required fields
                let is_ro = value
                    .get("is_ro")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| {
                        Error::InvalidArgs("map.value missing required 'is_ro' boolean".into())
                    })?;
                let is_io = value
                    .get("is_io")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| {
                        Error::InvalidArgs("map.value missing required 'is_io' boolean".into())
                    })?;

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
                // The "value" field holds the page address directly as a u64
                let page_str = kac.get("value").and_then(|v| v.as_str()).ok_or_else(|| {
                    Error::InvalidArgs("map_page capability missing 'value'".into())
                })?;
                let page_address_bytes =
                    parse_hex_u64(page_str).map_err(|err| Error::ParseHex {
                        field: "map_page value".into(),
                        value: page_str.into(),
                        source: err,
                    })?;

                // Convert byte address to page number (>> 12)
                let page = page_address_bytes >> 12;

                capabilities.push(KernelCapability::MapPage(page));
            }
            "map_region" => {
                // The "value" field holds an array of region descriptor objects
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

                    let region_type_u64 = region_obj
                        .get("region_type")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| {
                            Error::InvalidArgs(format!(
                                "map_region value[{}] missing 'region_type'",
                                idx
                            ))
                        })?;

                    if region_type_u64 > 63 {
                        return Err(Error::InvalidArgs(format!(
                            "map_region value[{}].region_type must be in range 0-63",
                            idx
                        )));
                    }
                    let region_type = region_type_u64 as u8;

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
                    let irq0_u64 = value_array[0].as_u64().ok_or_else(|| {
                        Error::InvalidArgs("irq_pair value[0] must be a number or null".into())
                    })?;
                    if irq0_u64 > 1023 {
                        return Err(Error::InvalidArgs(
                            "irq_pair value[0] must be in range 0-1023".into(),
                        ));
                    }
                    irq0_u64 as u16
                };

                let irq1 = if value_array[1].is_null() {
                    0x3FF
                } else {
                    let irq1_u64 = value_array[1].as_u64().ok_or_else(|| {
                        Error::InvalidArgs("irq_pair value[1] must be a number or null".into())
                    })?;
                    if irq1_u64 > 1023 {
                        return Err(Error::InvalidArgs(
                            "irq_pair value[1] must be in range 0-1023".into(),
                        ));
                    }
                    irq1_u64 as u16
                };

                capabilities.push(KernelCapability::IrqPair([irq0, irq1]));
            }
            "application_type" => {
                let app_type_u64 = kac.get("value").and_then(|v| v.as_u64()).ok_or_else(|| {
                    Error::InvalidArgs("application_type capability missing 'value'".into())
                })?;

                if app_type_u64 > 7 {
                    return Err(Error::InvalidArgs(
                        "application_type must be in range 0-7".into(),
                    ));
                }
                let app_type = app_type_u64 as u16;

                capabilities.push(KernelCapability::ApplicationType(app_type));
            }
            "min_kernel_version" => {
                // Accepts both numeric and hex string values
                let value = kac.get("value").ok_or_else(|| {
                    Error::InvalidArgs("min_kernel_version capability missing 'value'".into())
                })?;

                let min_kernel = if let Some(num) = value.as_u64() {
                    // Numeric value
                    num
                } else if let Some(s) = value.as_str() {
                    // Hex string value
                    parse_hex_u64(s).map_err(|err| Error::ParseHex {
                        field: "min_kernel_version".into(),
                        value: s.into(),
                        source: err,
                    })?
                } else {
                    return Err(Error::InvalidArgs(
                        "min_kernel_version value must be integer or hex string".into(),
                    ));
                };

                capabilities.push(KernelCapability::MinKernelVersion(min_kernel));
            }
            "handle_table_size" => {
                let handle_table_size_u64 =
                    kac.get("value").and_then(|v| v.as_u64()).ok_or_else(|| {
                        Error::InvalidArgs("handle_table_size capability missing 'value'".into())
                    })?;

                if handle_table_size_u64 > 1023 {
                    return Err(Error::InvalidArgs(
                        "handle_table_size must be in range 0-1023".into(),
                    ));
                }
                let handle_table_size = handle_table_size_u64 as u16;

                capabilities.push(KernelCapability::HandleTableSize(handle_table_size));
            }
            "debug_flags" => {
                // The "value" object holds the debug flag booleans
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

                // Mutual exclusion: at most one of the three flags may be set
                let flags_set =
                    (allow_debug as u8) + (force_debug_prod as u8) + (force_debug as u8);
                if flags_set > 1 {
                    return Err(Error::InvalidArgs(
                        "Only one of allow_debug, force_debug_prod, or force_debug can be set"
                            .into(),
                    ));
                }

                capabilities.push(KernelCapability::DebugFlags {
                    allow_debug,
                    force_debug_prod,
                    force_debug,
                });
            }
            _ => {
                return Err(Error::InvalidArgs(format!(
                    "Unknown kernel capability type '{}'",
                    kac_type
                )));
            }
        }
    }

    Ok(capabilities)
}

/// Parse a hexadecimal string with optional 0x/0X prefix.
///
/// Accepts both prefixed ("0x1234") and unprefixed ("1234") hex strings.
fn parse_hex_u64(s: &str) -> std::result::Result<u64, ParseIntError> {
    let stripped = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    u64::from_str_radix(stripped, 16)
}
