//! Descriptor representation of kernel capabilities.
//!
//! [`KernelCapabilityDescriptor`] mirrors the JSON form of a kernel capability
//! (an adjacently tagged `{ "type": ..., "value": ... }` object) and converts
//! into the [`KernelCapability`] consumed by the `nx-object` builder. The
//! conversion is where descriptor-level encoding is undone: byte addresses are
//! lowered to page numbers and value ranges are validated.

use std::collections::BTreeMap;

use nx_object::write::npdm::{KernelCapability, MapRegionDescriptor};

use super::{
    Error, check_range,
    hex::{HexU64, U64OrHex},
};

/// IRQ sentinel used by the format to mark a slot as unused (10-bit all-ones).
const UNUSED_IRQ: u16 = 0x3FF;

/// A single kernel capability as written in the descriptor JSON.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "json-schema", schemars(
    example = serde_json::json!({
        "type": "kernel_flags",
        "value": { "highest_thread_priority": 0, "lowest_thread_priority": 63, "highest_cpu_id": 3, "lowest_cpu_id": 0 }
    }),
    example = serde_json::json!({ "type": "syscalls", "value": { "svcSendSyncRequest": "0x21" } }),
    example = serde_json::json!({
        "type": "map",
        "value": { "address": "0x1000000", "size": "0x2000", "is_ro": false, "is_io": true }
    }),
))]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum KernelCapabilityDescriptor {
    /// Allowed thread-priority range and CPU-core affinity mask for the process.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Thread-priority range and CPU-core mask.")
    )]
    KernelFlags(KernelFlagsValue),
    /// Allowed supervisor calls (SVCs), mapping each name to its SVC number.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Allowed supervisor calls (name to SVC number).")
    )]
    Syscalls(BTreeMap<String, U64OrHex>),
    /// Maps a physical memory range into the process; addresses are byte values
    /// that are lowered to page numbers.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Map a physical memory range (byte addresses).")
    )]
    Map(MapValue),
    /// Maps a single physical page into the process; the value is a byte address.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Map a single physical page (byte address).")
    )]
    MapPage(HexU64),
    /// Maps up to three predefined memory regions.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Map up to three predefined regions.")
    )]
    MapRegion(Vec<MapRegionValue>),
    /// Allows up to two hardware interrupt (IRQ) numbers; `null` leaves a slot
    /// unused.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Allow up to two IRQ numbers (null = unused).")
    )]
    IrqPair([Option<u16>; 2]),
    /// Process application type (0-7), e.g. system module, application or applet.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Application type (0-7).")
    )]
    ApplicationType(u16),
    /// Minimum kernel version the process requires (packed major/minor).
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Minimum required kernel version.")
    )]
    MinKernelVersion(U64OrHex),
    /// Maximum number of kernel handles the process may hold.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Maximum kernel handle count.")
    )]
    HandleTableSize(u16),
    /// Debug permissions; at most one flag may be set.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Debug permissions (mutually exclusive).")
    )]
    DebugFlags(DebugFlagsValue),
}

/// Value object for [`KernelCapabilityDescriptor::KernelFlags`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "json-schema", schemars(example = serde_json::json!({
    "highest_thread_priority": 0,
    "lowest_thread_priority": 63,
    "highest_cpu_id": 3,
    "lowest_cpu_id": 0
})))]
pub struct KernelFlagsValue {
    /// Numerically highest priority value the process may use (0-63; 0 is the
    /// highest scheduling priority).
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Highest priority value allowed (0-63; 0 = highest).")
    )]
    pub highest_thread_priority: u8,
    /// Numerically lowest priority value the process may use (0-63).
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Lowest priority value allowed (0-63).")
    )]
    pub lowest_thread_priority: u8,
    /// Highest CPU core ID the process may run on.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Highest CPU core ID allowed.")
    )]
    pub highest_cpu_id: u8,
    /// Lowest CPU core ID the process may run on.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Lowest CPU core ID allowed.")
    )]
    pub lowest_cpu_id: u8,
}

/// Value object for [`KernelCapabilityDescriptor::Map`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "json-schema", schemars(example = serde_json::json!({
    "address": "0x1000000",
    "size": "0x2000",
    "is_ro": false,
    "is_io": true
})))]
pub struct MapValue {
    /// Physical byte address to map (page-aligned; lowered to a page number).
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Physical byte address to map (page-aligned).")
    )]
    pub address: HexU64,
    /// Mapping size in bytes (a multiple of the page size).
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Mapping size in bytes.")
    )]
    pub size: HexU64,
    /// Map the range as read-only (defaults to `false`).
    #[cfg_attr(feature = "json-schema", schemars(description = "Map as read-only."))]
    #[serde(default)]
    pub is_ro: bool,
    /// Map the range as device/MMIO memory rather than normal memory (defaults
    /// to `false`).
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Map as device/MMIO memory.")
    )]
    #[serde(default)]
    pub is_io: bool,
}

/// Value object for an entry of [`KernelCapabilityDescriptor::MapRegion`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "json-schema", schemars(example = serde_json::json!({
    "region_type": 1,
    "is_ro": true
})))]
pub struct MapRegionValue {
    /// Predefined memory-region selector (0-63).
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Memory-region selector (0-63).")
    )]
    pub region_type: u8,
    /// Map the region as read-only.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Map the region as read-only.")
    )]
    pub is_ro: bool,
}

/// Value object for [`KernelCapabilityDescriptor::DebugFlags`].
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "json-schema", schemars(example = serde_json::json!({ "allow_debug": true })))]
pub struct DebugFlagsValue {
    /// Permit a debugger to attach to this process.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Permit a debugger to attach.")
    )]
    #[serde(default)]
    pub allow_debug: bool,
    /// Force the process debuggable only on production units.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Force debuggable on production units.")
    )]
    #[serde(default)]
    pub force_debug_prod: bool,
    /// Force the process to be debuggable.
    #[cfg_attr(
        feature = "json-schema",
        schemars(description = "Force the process debuggable.")
    )]
    #[serde(default)]
    pub force_debug: bool,
}

impl TryFrom<KernelCapabilityDescriptor> for KernelCapability {
    type Error = Error;

    /// Convert a descriptor capability into its `nx-object` representation,
    /// validating value ranges and lowering byte addresses to page numbers.
    fn try_from(descriptor: KernelCapabilityDescriptor) -> Result<Self, Self::Error> {
        let capability = match descriptor {
            KernelCapabilityDescriptor::KernelFlags(value) => {
                check_range(
                    "kernel_flags.highest_thread_priority",
                    value.highest_thread_priority.into(),
                    63,
                )?;
                check_range(
                    "kernel_flags.lowest_thread_priority",
                    value.lowest_thread_priority.into(),
                    63,
                )?;
                KernelCapability::KernelFlags {
                    highest_thread_priority: value.highest_thread_priority,
                    lowest_thread_priority: value.lowest_thread_priority,
                    highest_cpu_id: value.highest_cpu_id,
                    lowest_cpu_id: value.lowest_cpu_id,
                }
            }
            KernelCapabilityDescriptor::Syscalls(map) => KernelCapability::Syscalls(
                map.into_iter().map(|(name, id)| (name, id.get())).collect(),
            ),
            KernelCapabilityDescriptor::Map(value) => KernelCapability::Map {
                // Descriptor addresses and sizes are byte values; the format
                // stores them as page numbers.
                address: value.address.get() >> 12,
                size: value.size.get() >> 12,
                is_ro: value.is_ro,
                is_io: value.is_io,
            },
            KernelCapabilityDescriptor::MapPage(page) => {
                KernelCapability::MapPage(page.get() >> 12)
            }
            KernelCapabilityDescriptor::MapRegion(regions) => {
                if regions.len() > 3 {
                    return Err(Error::TooManyMapRegions {
                        count: regions.len(),
                    });
                }
                let mut descriptors = Vec::with_capacity(regions.len());
                for region in regions {
                    check_range("map_region[].region_type", region.region_type.into(), 63)?;
                    descriptors.push(MapRegionDescriptor {
                        region_type: region.region_type,
                        is_ro: region.is_ro,
                    });
                }
                KernelCapability::MapRegion(descriptors)
            }
            KernelCapabilityDescriptor::IrqPair(pair) => {
                let mut irqs = [0u16; 2];
                for (slot, entry) in irqs.iter_mut().zip(pair) {
                    *slot = match entry {
                        Some(irq) => {
                            check_range("irq_pair[]", irq.into(), 1023)?;
                            irq
                        }
                        None => UNUSED_IRQ,
                    };
                }
                KernelCapability::IrqPair(irqs)
            }
            KernelCapabilityDescriptor::ApplicationType(app_type) => {
                check_range("application_type", app_type.into(), 7)?;
                KernelCapability::ApplicationType(app_type)
            }
            KernelCapabilityDescriptor::MinKernelVersion(value) => {
                KernelCapability::MinKernelVersion(value.get())
            }
            KernelCapabilityDescriptor::HandleTableSize(size) => {
                check_range("handle_table_size", size.into(), 1023)?;
                KernelCapability::HandleTableSize(size)
            }
            KernelCapabilityDescriptor::DebugFlags(value) => {
                let flags_set = u8::from(value.allow_debug)
                    + u8::from(value.force_debug_prod)
                    + u8::from(value.force_debug);
                if flags_set > 1 {
                    return Err(Error::ConflictingDebugFlags);
                }
                KernelCapability::DebugFlags {
                    allow_debug: value.allow_debug,
                    force_debug_prod: value.force_debug_prod,
                    force_debug: value.force_debug,
                }
            }
        };

        Ok(capability)
    }
}
