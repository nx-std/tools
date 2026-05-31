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
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum KernelCapabilityDescriptor {
    /// Thread priority range and CPU affinity bounds.
    KernelFlags(KernelFlagsValue),
    /// Allowed system calls, mapping name to numeric ID.
    Syscalls(BTreeMap<String, U64OrHex>),
    /// Memory region mapping with permissions; addresses are byte addresses.
    Map(MapValue),
    /// Single-page mapping permission; the value is a byte address.
    MapPage(HexU64),
    /// Up to three predefined memory region descriptors.
    MapRegion(Vec<MapRegionValue>),
    /// A pair of allowed IRQ numbers; `null` marks an unused slot.
    IrqPair([Option<u16>; 2]),
    /// Application type (applet, application, system module).
    ApplicationType(u16),
    /// Minimum required kernel version.
    MinKernelVersion(U64OrHex),
    /// Maximum number of kernel handles.
    HandleTableSize(u16),
    /// Debug permission flags (mutually exclusive).
    DebugFlags(DebugFlagsValue),
}

/// Value object for [`KernelCapabilityDescriptor::KernelFlags`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub struct KernelFlagsValue {
    /// Highest allowed thread priority (0-63, higher = lower priority).
    pub highest_thread_priority: u8,
    /// Lowest allowed thread priority (0-63).
    pub lowest_thread_priority: u8,
    /// Highest allowed CPU core ID.
    pub highest_cpu_id: u8,
    /// Lowest allowed CPU core ID.
    pub lowest_cpu_id: u8,
}

/// Value object for [`KernelCapabilityDescriptor::Map`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub struct MapValue {
    /// Byte address to map.
    pub address: HexU64,
    /// Size of the mapping in bytes.
    pub size: HexU64,
    /// Read-only flag (defaults to `false`).
    #[serde(default)]
    pub is_ro: bool,
    /// I/O flag (device memory vs normal memory); defaults to `false`.
    #[serde(default)]
    pub is_io: bool,
}

/// Value object for an entry of [`KernelCapabilityDescriptor::MapRegion`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub struct MapRegionValue {
    /// Region type (0-63).
    pub region_type: u8,
    /// Read-only flag.
    pub is_ro: bool,
}

/// Value object for [`KernelCapabilityDescriptor::DebugFlags`].
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub struct DebugFlagsValue {
    /// Allow debugging this process.
    #[serde(default)]
    pub allow_debug: bool,
    /// Force production debugging mode.
    #[serde(default)]
    pub force_debug_prod: bool,
    /// Force debugging this process.
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
