//! Lowering of a package's inline `[package.metadata.nx.nsp.npdm]` block into the
//! JSON descriptor the NPDM builder reads.
//!
//! The manifest spells values the way a person writes them — `0x`-prefixed hex,
//! dotted kernel versions, syscalls by name — and the descriptor wants them
//! normalised. Every conversion that can fail on user input happens here, so the
//! error names the manifest key rather than a position in the generated JSON.

use super::metadata::InlineNpdm;

/// Lower the inline `[package.metadata.nx.nsp.npdm]` block into descriptor JSON.
///
/// # Errors
///
/// Returns an error if a declared syscall name is not one the kernel defines, or
/// if `kernel_version` is dotted but not `major.minor` with a minor below 16.
pub fn convert_inline_npdm_to_json(
    inline: &InlineNpdm,
) -> Result<serde_json::Value, ConvertNpdmError> {
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
                    return Err(ConvertNpdmError::UnknownSyscall {
                        name: syscall_name.clone(),
                    });
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
                    ConvertNpdmError::InvalidKernelVersion {
                        value: version.clone(),
                    }
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

/// Errors produced while lowering the inline NPDM metadata into descriptor JSON.
#[derive(Debug, thiserror::Error)]
pub enum ConvertNpdmError {
    /// A declared syscall is not one the kernel defines.
    ///
    /// Names must match the kernel ABI exactly, so this is usually a typo or a
    /// difference in capitalisation. Holds the offending name.
    #[error("unknown syscall '{name}'; names must match the kernel ABI, such as `SetHeapSize`")]
    UnknownSyscall {
        /// The name that matched no syscall.
        name: String,
    },

    /// `kernel_version` is dotted but not a `major.minor` pair the format accepts.
    ///
    /// The minor component is encoded in four bits, so it must be below 16. Holds
    /// the offending value.
    #[error("invalid kernel_version '{value}'; expected 'major.minor' with a minor below 16")]
    InvalidKernelVersion {
        /// The value that could not be encoded.
        value: String,
    },
}

/// Strip a `0x`/`0X` prefix from a hex string, if present.
///
/// Normalizes hex strings from TOML to bare hex digits for the NPDM descriptor.
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
    use super::{ConvertNpdmError, InlineNpdm, convert_inline_npdm_to_json};

    /// Build an `InlineNpdm` the way the build command does: by deserializing the
    /// `[package.metadata.nx.nsp.npdm]` block. `extra` is merged over the minimum
    /// set of required fields.
    fn inline_npdm(extra: serde_json::Value) -> InlineNpdm {
        let mut value = serde_json::json!({
            "name": "demo",
            "main_thread_stack_size": "0x100000",
            "main_thread_priority": 44,
            "main_thread_core_number": 0,
            "address_space_type": 3,
            "is_64_bit": true,
            "optimize_memory_allocation": false,
            "disable_device_address_space_merge": false,
            "program_id": "0x0100000000010000",
        });
        let (Some(object), Some(extra)) = (value.as_object_mut(), extra.as_object()) else {
            panic!("both the base descriptor and the overlay must be JSON objects");
        };
        for (key, val) in extra {
            object.insert(key.clone(), val.clone());
        }
        serde_json::from_value(value).expect("the descriptor fixture should deserialize")
    }

    #[test]
    fn convert_inline_npdm_to_json_with_an_unknown_syscall_fails() {
        //* Given
        let inline = inline_npdm(serde_json::json!({
            "kernel_capabilities": { "enable_system_calls": ["NotASyscall"] }
        }));

        //* When
        let result = convert_inline_npdm_to_json(&inline);

        //* Then
        assert!(
            matches!(result, Err(ConvertNpdmError::UnknownSyscall { ref name }) if name == "NotASyscall"),
            "the unmatched name should be carried on the error, got {result:?}"
        );
    }

    #[test]
    fn convert_inline_npdm_to_json_with_an_out_of_range_kernel_version_fails() {
        //* Given
        // The minor component is encoded in four bits, so 16 does not fit.
        let inline = inline_npdm(serde_json::json!({
            "kernel_capabilities": { "kernel_version": "3.16" }
        }));

        //* When
        let result = convert_inline_npdm_to_json(&inline);

        //* Then
        assert!(
            matches!(result, Err(ConvertNpdmError::InvalidKernelVersion { ref value }) if value == "3.16"),
            "the rejected version should be carried on the error, got {result:?}"
        );
    }

    #[test]
    fn convert_inline_npdm_to_json_with_a_known_syscall_succeeds() {
        //* Given
        let inline = inline_npdm(serde_json::json!({
            "kernel_capabilities": { "enable_system_calls": ["SetHeapSize"] }
        }));

        //* When
        let result = convert_inline_npdm_to_json(&inline);

        //* Then
        assert!(
            result.is_ok(),
            "a name matching the kernel ABI should convert, got {result:?}"
        );
    }
}
