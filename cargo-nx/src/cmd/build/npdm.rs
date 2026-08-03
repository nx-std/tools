//! Conversion of a package's inline `[package.metadata.nx.nsp.npdm]` block into the
//! process descriptor the NPDM builder takes.
//!
//! The manifest spells values the way a person writes them — `0x`-prefixed hex,
//! dotted kernel versions, syscalls by name — and the descriptor wants them decoded.
//! Everything that can fail on user input fails here, naming the manifest key that
//! caused it, so no later stage has to report a problem in terms the author never
//! wrote.

use cargo_nx::npdm::{
    FilesystemAccessDescriptor, HexU64, KernelCapabilityDescriptor, KernelFlagsValue,
    NpdmDescriptor,
};

use super::metadata::{InlineKernelCapabilities, InlineNpdm};

/// Convert the inline block into a process descriptor.
///
/// # Errors
///
/// Returns an error if a hexadecimal field cannot be decoded, if the kernel-flags
/// block is only partly specified, if a declared syscall is not one the kernel
/// defines, or if `kernel_version` is dotted but not `major.minor` with a minor
/// below 16.
pub fn to_descriptor(inline: &InlineNpdm) -> Result<NpdmDescriptor, ConvertNpdmError> {
    let program_id = parse_hex("program_id", &inline.program_id)?;

    Ok(NpdmDescriptor {
        name: inline.name.clone(),
        program_id,
        main_thread_stack_size: parse_hex(
            "main_thread_stack_size",
            &inline.main_thread_stack_size,
        )?,
        main_thread_priority: inline.main_thread_priority,
        default_cpu_id: inline.main_thread_core_number,
        version: u64::from(inline.version).into(),
        address_space_type: inline.address_space_type,
        is_64_bit: inline.is_64_bit,
        optimize_memory_allocation: inline.optimize_memory_allocation,
        disable_device_address_space_merge: inline.disable_device_address_space_merge,
        enable_alias_region_extra_size: false,
        prevent_code_reads: false,
        is_retail: inline.is_retail,
        pool_partition: 0,
        // The inline block authorizes exactly the one program it declares, rather
        // than a range: a manifest that wanted a range would use `npdm_json`.
        program_id_range_min: program_id,
        program_id_range_max: program_id,
        filesystem_access: FilesystemAccessDescriptor {
            permissions: match inline.fs_access_control.as_ref() {
                Some(access) => parse_hex("fs_access_control.flags", &access.flags)?,
                None => 0u64.into(),
            },
            content_owner_ids: Vec::new(),
            save_data_owner_ids: Vec::new(),
        },
        service_host: inline
            .service_access_control
            .as_ref()
            .map(|access| access.hosted_services.clone())
            .unwrap_or_default(),
        service_access: inline
            .service_access_control
            .as_ref()
            .map(|access| access.accessed_services.clone())
            .unwrap_or_default(),
        kernel_capabilities: match inline.kernel_capabilities.as_ref() {
            Some(capabilities) => to_capabilities(capabilities)?,
            None => Vec::new(),
        },
    })
}

/// Decode a hexadecimal manifest field, naming it if it cannot be decoded.
fn parse_hex(field: &'static str, value: &str) -> Result<HexU64, ConvertNpdmError> {
    value
        .parse()
        .map_err(|source| ConvertNpdmError::InvalidHex {
            field,
            value: value.to_owned(),
            source,
        })
}

/// Convert the inline kernel-capability block into descriptor entries.
fn to_capabilities(
    inline: &InlineKernelCapabilities,
) -> Result<Vec<KernelCapabilityDescriptor>, ConvertNpdmError> {
    let mut capabilities = Vec::new();

    // The four bounds describe one range, and the descriptor has no way to express
    // a partial one, so either all of them are given or none is.
    let flags = [
        ("highest_priority", inline.highest_priority),
        ("lowest_priority", inline.lowest_priority),
        ("max_core_number", inline.max_core_number),
        ("min_core_number", inline.min_core_number),
    ];
    let missing: Vec<&'static str> = flags
        .iter()
        .filter(|(_, value)| value.is_none())
        .map(|(key, _)| *key)
        .collect();
    match missing.len() {
        0 => capabilities.push(KernelCapabilityDescriptor::KernelFlags(KernelFlagsValue {
            highest_thread_priority: inline.highest_priority.unwrap_or_default(),
            lowest_thread_priority: inline.lowest_priority.unwrap_or_default(),
            highest_cpu_id: inline.max_core_number.unwrap_or_default(),
            lowest_cpu_id: inline.min_core_number.unwrap_or_default(),
        })),
        4 => {}
        _ => {
            return Err(ConvertNpdmError::PartialKernelFlags {
                missing: missing.join(", "),
            });
        }
    }

    if !inline.enable_system_calls.is_empty() {
        let mut syscalls = std::collections::BTreeMap::new();
        for name in &inline.enable_system_calls {
            let Some(id) = syscall_name_to_id(name) else {
                return Err(ConvertNpdmError::UnknownSyscall { name: name.clone() });
            };
            syscalls.insert(name.clone(), id.into());
        }
        capabilities.push(KernelCapabilityDescriptor::Syscalls(syscalls));
    }

    if let Some(version) = inline.kernel_version.as_ref() {
        let encoded = parse_kernel_version(version).ok_or_else(|| {
            ConvertNpdmError::InvalidKernelVersion {
                value: version.clone(),
            }
        })?;
        capabilities.push(KernelCapabilityDescriptor::MinKernelVersion(encoded.into()));
    }

    Ok(capabilities)
}

/// Errors produced while converting the inline NPDM metadata into a descriptor.
#[derive(Debug, thiserror::Error)]
pub enum ConvertNpdmError {
    /// A hexadecimal field could not be decoded.
    ///
    /// Holds the manifest key so the report names what the author wrote, and
    /// accepts a `0x` prefix, so this means the value is not hexadecimal at all.
    #[error("invalid hexadecimal value '{value}' for `{field}`")]
    InvalidHex {
        /// The manifest key that held the value.
        field: &'static str,
        /// The value that failed to decode.
        value: String,
        #[source]
        source: std::num::ParseIntError,
    },

    /// The kernel-flags bounds were only partly given.
    ///
    /// The four keys describe a single range that the descriptor cannot express
    /// partially, so they are set together or left out together.
    #[error("incomplete kernel flags: `{missing}` must be set alongside the others")]
    PartialKernelFlags {
        /// The keys that were left out, comma-separated.
        missing: String,
    },

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

/// The kernel syscall number a name refers to, or `None` if the kernel defines
/// no such syscall.
fn syscall_name_to_id(name: &str) -> Option<u64> {
    match name {
        "SetHeapSize" => Some(0x1),
        "SetMemoryPermission" => Some(0x2),
        "SetMemoryAttribute" => Some(0x3),
        "MapMemory" => Some(0x4),
        "UnmapMemory" => Some(0x5),
        "QueryMemory" => Some(0x6),
        "ExitProcess" => Some(0x7),
        "CreateThread" => Some(0x8),
        "StartThread" => Some(0x9),
        "ExitThread" => Some(0xa),
        "SleepThread" => Some(0xb),
        "GetThreadPriority" => Some(0xc),
        "SetThreadPriority" => Some(0xd),
        "GetThreadCoreMask" => Some(0xe),
        "SetThreadCoreMask" => Some(0xf),
        "GetCurrentProcessorNumber" => Some(0x10),
        "SignalEvent" => Some(0x11),
        "ClearEvent" => Some(0x12),
        "MapSharedMemory" => Some(0x13),
        "UnmapSharedMemory" => Some(0x14),
        "CreateTransferMemory" => Some(0x15),
        "CloseHandle" => Some(0x16),
        "ResetSignal" => Some(0x17),
        "WaitSynchronization" => Some(0x18),
        "CancelSynchronization" => Some(0x19),
        "ArbitrateLock" => Some(0x1a),
        "ArbitrateUnlock" => Some(0x1b),
        "WaitProcessWideKeyAtomic" => Some(0x1c),
        "SignalProcessWideKey" => Some(0x1d),
        "GetSystemTick" => Some(0x1e),
        "ConnectToNamedPort" => Some(0x1f),
        "SendSyncRequestLight" => Some(0x20),
        "SendSyncRequest" => Some(0x21),
        "SendSyncRequestWithUserBuffer" => Some(0x22),
        "SendAsyncRequestWithUserBuffer" => Some(0x23),
        "GetProcessId" => Some(0x24),
        "GetThreadId" => Some(0x25),
        "Break" => Some(0x26),
        "OutputDebugString" => Some(0x27),
        "ReturnFromException" => Some(0x28),
        "GetInfo" => Some(0x29),
        "FlushEntireDataCache" => Some(0x2a),
        "FlushDataCache" => Some(0x2b),
        "MapPhysicalMemory" => Some(0x2c),
        "UnmapPhysicalMemory" => Some(0x2d),
        "GetDebugFutureThreadInfo" => Some(0x2e),
        "GetLastThreadInfo" => Some(0x2f),
        "GetResourceLimitLimitValue" => Some(0x30),
        "GetResourceLimitCurrentValue" => Some(0x31),
        "SetThreadActivity" => Some(0x32),
        "GetThreadContext3" => Some(0x33),
        "WaitForAddress" => Some(0x34),
        "SignalToAddress" => Some(0x35),
        "SynchronizePreemptionState" => Some(0x36),
        "GetResourceLimitPeakValue" => Some(0x37),
        "CreateIoPool" => Some(0x39),
        "CreateIoRegion" => Some(0x3a),
        "KernelDebug" => Some(0x3c),
        "ChangeKernelTraceState" => Some(0x3d),
        "CreateSession" => Some(0x40),
        "AcceptSession" => Some(0x41),
        "ReplyAndReceiveLight" => Some(0x42),
        "ReplyAndReceive" => Some(0x43),
        "ReplyAndReceiveWithUserBuffer" => Some(0x44),
        "CreateEvent" => Some(0x45),
        "MapIoRegion" => Some(0x46),
        "UnmapIoRegion" => Some(0x47),
        "MapPhysicalMemoryUnsafe" => Some(0x48),
        "UnmapPhysicalMemoryUnsafe" => Some(0x49),
        "SetUnsafeLimit" => Some(0x4a),
        "CreateCodeMemory" => Some(0x4b),
        "ControlCodeMemory" => Some(0x4c),
        "SleepSystem" => Some(0x4d),
        "ReadWriteRegister" => Some(0x4e),
        "SetProcessActivity" => Some(0x4f),
        "CreateSharedMemory" => Some(0x50),
        "MapTransferMemory" => Some(0x51),
        "UnmapTransferMemory" => Some(0x52),
        "CreateInterruptEvent" => Some(0x53),
        "QueryPhysicalAddress" => Some(0x54),
        "QueryIoMapping" => Some(0x55),
        "CreateDeviceAddressSpace" => Some(0x56),
        "AttachDeviceAddressSpace" => Some(0x57),
        "DetachDeviceAddressSpace" => Some(0x58),
        "MapDeviceAddressSpaceByForce" => Some(0x59),
        "MapDeviceAddressSpaceAligned" => Some(0x5a),
        "MapDeviceAddressSpace" => Some(0x5b),
        "UnmapDeviceAddressSpace" => Some(0x5c),
        "InvalidateProcessDataCache" => Some(0x5d),
        "StoreProcessDataCache" => Some(0x5e),
        "FlushProcessDataCache" => Some(0x5f),
        "DebugActiveProcess" => Some(0x60),
        "BreakDebugProcess" => Some(0x61),
        "TerminateDebugProcess" => Some(0x62),
        "GetDebugEvent" => Some(0x63),
        "ContinueDebugEvent" => Some(0x64),
        "GetProcessList" => Some(0x65),
        "GetThreadList" => Some(0x66),
        "GetDebugThreadContext" => Some(0x67),
        "SetDebugThreadContext" => Some(0x68),
        "QueryDebugProcessMemory" => Some(0x69),
        "ReadDebugProcessMemory" => Some(0x6a),
        "WriteDebugProcessMemory" => Some(0x6b),
        "SetHardwareBreakPoint" => Some(0x6c),
        "GetDebugThreadParam" => Some(0x6d),
        "GetSystemInfo" => Some(0x6f),
        "CreatePort" => Some(0x70),
        "ManageNamedPort" => Some(0x71),
        "ConnectToPort" => Some(0x72),
        "SetProcessMemoryPermission" => Some(0x73),
        "MapProcessMemory" => Some(0x74),
        "UnmapProcessMemory" => Some(0x75),
        "QueryProcessMemory" => Some(0x76),
        "MapProcessCodeMemory" => Some(0x77),
        "UnmapProcessCodeMemory" => Some(0x78),
        "CreateProcess" => Some(0x79),
        "StartProcess" => Some(0x7a),
        "TerminateProcess" => Some(0x7b),
        "GetProcessInfo" => Some(0x7c),
        "CreateResourceLimit" => Some(0x7d),
        "SetResourceLimitLimitValue" => Some(0x7e),
        "CallSecureMonitor" => Some(0x7f),
        "MapInsecurePhysicalMemory" => Some(0x90),
        "UnmapInsecurePhysicalMemory" => Some(0x91),
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
fn parse_kernel_version(version: &str) -> Option<u64> {
    // A bare hex string is already the encoded form and passes through.
    let Some((major, minor)) = version.split_once('.') else {
        return u64::from_str_radix(version, 16).ok();
    };

    let major: u64 = major.parse().ok()?;
    let minor: u64 = minor.parse().ok()?;

    // The minor component occupies four bits, so it cannot reach 16.
    if minor >= 16 {
        return None;
    }

    Some((major << 4) | minor)
}

#[cfg(test)]
mod tests {
    use super::{ConvertNpdmError, InlineNpdm, to_descriptor};

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
    fn to_descriptor_with_an_unknown_syscall_fails() {
        //* Given
        let inline = inline_npdm(serde_json::json!({
            "kernel_capabilities": { "enable_system_calls": ["NotASyscall"] }
        }));

        //* When
        let result = to_descriptor(&inline);

        //* Then
        assert!(
            matches!(result, Err(ConvertNpdmError::UnknownSyscall { ref name }) if name == "NotASyscall"),
            "the unmatched name should be carried on the error, got {result:?}"
        );
    }

    #[test]
    fn to_descriptor_with_an_out_of_range_kernel_version_fails() {
        //* Given
        // The minor component is encoded in four bits, so 16 does not fit.
        let inline = inline_npdm(serde_json::json!({
            "kernel_capabilities": { "kernel_version": "3.16" }
        }));

        //* When
        let result = to_descriptor(&inline);

        //* Then
        assert!(
            matches!(result, Err(ConvertNpdmError::InvalidKernelVersion { ref value }) if value == "3.16"),
            "the rejected version should be carried on the error, got {result:?}"
        );
    }

    #[test]
    fn to_descriptor_with_partial_kernel_flags_names_the_missing_keys() {
        //* Given
        // The four bounds describe one range the descriptor cannot express partly.
        let inline = inline_npdm(serde_json::json!({
            "kernel_capabilities": { "highest_priority": 0 }
        }));

        //* When
        let result = to_descriptor(&inline);

        //* Then
        assert!(
            matches!(
                result,
                Err(ConvertNpdmError::PartialKernelFlags { ref missing })
                    if missing.contains("lowest_priority")
            ),
            "the report should name the manifest keys left out, got {result:?}"
        );
    }

    #[test]
    fn to_descriptor_with_all_kernel_flags_succeeds() {
        //* Given
        let inline = inline_npdm(serde_json::json!({
            "kernel_capabilities": {
                "highest_priority": 0,
                "lowest_priority": 63,
                "max_core_number": 3,
                "min_core_number": 0
            }
        }));

        //* When
        let result = to_descriptor(&inline);

        //* Then
        assert!(
            result.is_ok(),
            "a fully specified kernel_flags block should convert, got {result:?}"
        );
    }

    #[test]
    fn to_descriptor_with_a_known_syscall_succeeds() {
        //* Given
        let inline = inline_npdm(serde_json::json!({
            "kernel_capabilities": { "enable_system_calls": ["SetHeapSize"] }
        }));

        //* When
        let result = to_descriptor(&inline);

        //* Then
        assert!(
            result.is_ok(),
            "a name matching the kernel ABI should convert, got {result:?}"
        );
    }
}
