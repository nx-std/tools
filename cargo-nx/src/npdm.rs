//! Typed NPDM JSON descriptor.
//!
//! [`NpdmDescriptor`] is a faithful, `serde`-derived model of the `main.npdm`
//! JSON descriptor accepted by the build pipeline. It exists to give the
//! descriptor format a single entry-point type that can be deserialized,
//! schema-generated (behind the `json-schema` feature), and converted into the
//! `nx-object` builder inputs.
//!
//! The descriptor is the validation boundary: hexadecimal fields are decoded,
//! value ranges are checked, and byte addresses are lowered to page numbers
//! during the [`TryFrom`] conversion into the `nx-object` types, which then
//! trust their inputs.

mod hex;
mod kernel_capability;

pub use hex::{HexU64, U64OrHex};
pub use kernel_capability::KernelCapabilityDescriptor;
use nx_object::write::npdm::{
    AciData, AcidData, FilesystemAccess, KernelCapability, NpdmBuilder, NpdmMetadata,
    SaveDataOwnerId, ServiceAccess,
};

/// A complete NPDM descriptor as written in the `main.npdm` JSON file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub struct NpdmDescriptor {
    /// Program name (UTF-8, truncated to 16 bytes including the null terminator).
    pub name: String,

    /// Program (title) ID.
    #[serde(alias = "title_id")]
    pub program_id: HexU64,

    /// Main thread stack size in bytes (must fit in a `u32`).
    pub main_thread_stack_size: HexU64,

    /// Main thread priority (0-63, higher = lower priority).
    pub main_thread_priority: u8,

    /// CPU core the main thread starts on (0-3).
    pub default_cpu_id: u8,

    /// Program version (defaults to 0).
    #[serde(default, alias = "process_category")]
    pub version: HexU64,

    /// Address space type (0-3).
    pub address_space_type: u8,

    /// Whether the program is 64-bit.
    pub is_64_bit: bool,

    /// Optimize memory allocation flag.
    #[serde(default)]
    pub optimize_memory_allocation: bool,

    /// Disable device address space merge flag.
    #[serde(default)]
    pub disable_device_address_space_merge: bool,

    /// Enable alias region extra size flag.
    #[serde(default)]
    pub enable_alias_region_extra_size: bool,

    /// Prevent code reads flag.
    #[serde(default)]
    pub prevent_code_reads: bool,

    /// Whether the ACID is flagged for retail.
    pub is_retail: bool,

    /// Memory pool partition (0-3).
    pub pool_partition: u8,

    /// Lowest program ID the ACID permits.
    #[serde(alias = "title_id_range_min")]
    pub program_id_range_min: HexU64,

    /// Highest program ID the ACID permits.
    #[serde(alias = "title_id_range_max")]
    pub program_id_range_max: HexU64,

    /// Filesystem access control.
    pub filesystem_access: FilesystemAccessDescriptor,

    /// Services this program provides (hosts).
    #[serde(default)]
    pub service_host: Vec<String>,

    /// Services this program accesses as a client.
    #[serde(default)]
    pub service_access: Vec<String>,

    /// Kernel capabilities.
    pub kernel_capabilities: Vec<KernelCapabilityDescriptor>,
}

/// Filesystem access control section of the descriptor.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub struct FilesystemAccessDescriptor {
    /// Bitmask of permitted filesystem operations.
    pub permissions: HexU64,

    /// Content owner IDs (ACI0 list form).
    #[serde(default)]
    pub content_owner_ids: Vec<HexU64>,

    /// Save data owner IDs (ACI0 list form).
    #[serde(default)]
    pub save_data_owner_ids: Vec<SaveDataOwnerIdDescriptor>,
}

/// A single save-data owner ID entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub struct SaveDataOwnerIdDescriptor {
    /// Accessibility level (0-3).
    pub accessibility: u8,

    /// Owner ID.
    pub id: HexU64,
}

/// Errors produced while converting a [`NpdmDescriptor`] into `nx-object` inputs.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A descriptor field holds a value outside its permitted range.
    ///
    /// Range-constrained fields (thread priority, CPU id, IRQ numbers, the main
    /// thread stack size, …) are validated during conversion before reaching the
    /// `nx-object` builder.
    #[error("field '{field}' value {value} is out of range (0..={max})")]
    ValueOutOfRange {
        /// Name of the offending descriptor field.
        field: &'static str,
        /// The value that failed validation.
        value: u64,
        /// Inclusive maximum the field accepts.
        max: u64,
    },

    /// A `map_region` capability lists more region descriptors than the format allows.
    ///
    /// The kernel capability format encodes at most three region descriptors.
    #[error("'map_region' capability has {count} descriptors, but at most 3 are allowed")]
    TooManyMapRegions {
        /// Number of region descriptors that were supplied.
        count: usize,
    },

    /// A `debug_flags` capability sets more than one mutually-exclusive flag.
    ///
    /// At most one of `allow_debug`, `force_debug_prod`, or `force_debug` may be set.
    #[error("'debug_flags' capability sets more than one mutually-exclusive flag")]
    ConflictingDebugFlags,
}

impl NpdmDescriptor {
    /// Build a complete NPDM image from this descriptor.
    ///
    /// Validates and converts the descriptor into the `nx-object` builder inputs,
    /// then serializes the META/ACI0/ACID sections into the final image bytes.
    pub fn build(self) -> Result<Vec<u8>, Error> {
        let (metadata, aci, acid) = <(NpdmMetadata, AciData, AcidData)>::try_from(self)?;
        Ok(NpdmBuilder::new(metadata)
            .with_aci(aci)
            .with_acid(acid)
            .build())
    }
}

impl TryFrom<NpdmDescriptor> for (NpdmMetadata, AciData, AcidData) {
    type Error = Error;

    fn try_from(descriptor: NpdmDescriptor) -> Result<Self, Self::Error> {
        // Validate the scalar range-constrained fields up front.
        check_range(
            "main_thread_priority",
            descriptor.main_thread_priority.into(),
            63,
        )?;
        check_range("default_cpu_id", descriptor.default_cpu_id.into(), 3)?;
        check_range(
            "address_space_type",
            descriptor.address_space_type.into(),
            3,
        )?;
        check_range("pool_partition", descriptor.pool_partition.into(), 3)?;

        let stack_size = descriptor.main_thread_stack_size.get();
        check_range("main_thread_stack_size", stack_size, u64::from(u32::MAX))?;
        let main_thread_stack_size = stack_size as u32;

        // The legacy descriptor clamps the version to the u32 range.
        let version = descriptor.version.get().min(u64::from(u32::MAX)) as u32;

        let mut flags = ((descriptor.address_space_type & 3) << 1) | u8::from(descriptor.is_64_bit);
        flags |= u8::from(descriptor.optimize_memory_allocation) << 4;
        flags |= u8::from(descriptor.disable_device_address_space_merge) << 5;
        flags |= u8::from(descriptor.enable_alias_region_extra_size) << 6;
        flags |= u8::from(descriptor.prevent_code_reads) << 7;

        let title_id = descriptor.program_id.get();
        let metadata = NpdmMetadata {
            name: descriptor.name,
            title_id,
            version,
            main_thread_priority: descriptor.main_thread_priority,
            default_cpu_id: descriptor.default_cpu_id,
            main_thread_stack_size,
            flags,
        };

        // Access control is shared between the ACI0 and ACID sections.
        let filesystem_access = descriptor.filesystem_access.into_nx()?;
        let service_access =
            build_service_access(&descriptor.service_host, &descriptor.service_access);
        let kernel_capabilities = descriptor
            .kernel_capabilities
            .into_iter()
            .map(KernelCapability::try_from)
            .collect::<Result<Vec<KernelCapability>, Error>>()?;

        let aci = AciData {
            program_id: title_id,
            filesystem_access: filesystem_access.clone(),
            service_access: service_access.clone(),
            kernel_capabilities: kernel_capabilities.clone(),
        };

        let acid_flags =
            u32::from(descriptor.is_retail) | ((u32::from(descriptor.pool_partition) & 3) << 2);
        let acid = AcidData {
            program_id_min: descriptor.program_id_range_min.get(),
            program_id_max: descriptor.program_id_range_max.get(),
            filesystem_access,
            service_access,
            kernel_capabilities,
            flags: acid_flags,
        };

        Ok((metadata, aci, acid))
    }
}

impl FilesystemAccessDescriptor {
    /// Convert into the `nx-object` filesystem access control, validating entries.
    fn into_nx(self) -> Result<FilesystemAccess, Error> {
        let mut save_data_owner_ids = Vec::with_capacity(self.save_data_owner_ids.len());
        for entry in self.save_data_owner_ids {
            check_range(
                "save_data_owner_ids[].accessibility",
                entry.accessibility.into(),
                3,
            )?;
            save_data_owner_ids.push(SaveDataOwnerId {
                accessibility: entry.accessibility,
                id: entry.id.get(),
            });
        }

        Ok(FilesystemAccess {
            permissions: self.permissions.get(),
            content_owner_ids: self.content_owner_ids.iter().map(|id| id.get()).collect(),
            save_data_owner_ids,
            content_owner_id_min: 0,
            content_owner_id_max: 0,
            save_data_owner_id_min: 0,
            save_data_owner_id_max: 0,
        })
    }
}

/// Build the `nx-object` service access list from the host and client services.
///
/// Hosted services are flagged `is_host = true`; client services `false`.
fn build_service_access(hosts: &[String], clients: &[String]) -> ServiceAccess {
    let mut services = Vec::with_capacity(hosts.len() + clients.len());
    services.extend(hosts.iter().map(|name| (name.clone(), true)));
    services.extend(clients.iter().map(|name| (name.clone(), false)));
    ServiceAccess { services }
}

/// Validate that `value` does not exceed the inclusive maximum `max`.
fn check_range(field: &'static str, value: u64, max: u64) -> Result<(), Error> {
    if value > max {
        return Err(Error::ValueOutOfRange { field, value, max });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use nx_object::write::npdm::{AciData, AcidData, KernelCapability, NpdmMetadata};

    use super::{Error, NpdmDescriptor};

    /// Build a valid descriptor with the given `kernel_capabilities` JSON array.
    fn descriptor_with_capabilities(capabilities_json: &str) -> NpdmDescriptor {
        let json = format!(
            r#"{{
                "name": "Test",
                "program_id": "0x0100000000010000",
                "main_thread_stack_size": "0x100000",
                "main_thread_priority": 44,
                "default_cpu_id": 0,
                "address_space_type": 3,
                "is_64_bit": true,
                "is_retail": false,
                "pool_partition": 0,
                "program_id_range_min": "0x0",
                "program_id_range_max": "0xffffffffffffffff",
                "filesystem_access": {{ "permissions": "0xffffffffffffffff" }},
                "kernel_capabilities": {capabilities_json}
            }}"#
        );
        serde_json::from_str(&json).expect("descriptor JSON should deserialize")
    }

    #[test]
    fn try_from_with_minimal_descriptor_maps_metadata() {
        //* Given
        let descriptor = descriptor_with_capabilities("[]");

        //* When
        let result: Result<(NpdmMetadata, AciData, AcidData), Error> = descriptor.try_into();

        //* Then
        let (metadata, aci, acid) =
            result.expect("conversion should succeed for a valid descriptor");
        assert_eq!(
            metadata.title_id, 0x0100_0000_0001_0000,
            "title id should decode from program_id"
        );
        assert_eq!(
            metadata.flags, 0b111,
            "flags should pack address_space_type=3 and is_64_bit=true"
        );
        assert_eq!(
            metadata.main_thread_stack_size, 0x10_0000,
            "stack size should decode from the hexadecimal string"
        );
        assert_eq!(metadata.version, 0, "version should default to 0");
        assert_eq!(
            aci.program_id, metadata.title_id,
            "aci program id should mirror the title id"
        );
        assert_eq!(
            acid.program_id_max,
            u64::MAX,
            "acid maximum range should decode from hex"
        );
        assert_eq!(
            acid.flags, 0,
            "acid flags should pack is_retail=false and pool_partition=0"
        );
    }

    #[test]
    fn try_from_with_map_capability_lowers_bytes_to_pages() {
        //* Given
        let descriptor = descriptor_with_capabilities(
            r#"[{ "type": "map", "value": { "address": "0x1000000", "size": "0x2000", "is_ro": false, "is_io": true } }]"#,
        );

        //* When
        let result: Result<(NpdmMetadata, AciData, AcidData), Error> = descriptor.try_into();

        //* Then
        let (_, aci, _) = result.expect("conversion should succeed");
        let capability = aci
            .kernel_capabilities
            .first()
            .expect("one kernel capability should be present");
        assert!(
            matches!(
                capability,
                KernelCapability::Map { address, size, is_ro: false, is_io: true }
                    if *address == 0x1000 && *size == 0x2
            ),
            "byte address/size should be lowered to page numbers, got {capability:?}"
        );
    }

    #[test]
    fn try_from_with_out_of_range_priority_fails() {
        //* Given
        let mut descriptor = descriptor_with_capabilities("[]");
        descriptor.main_thread_priority = 64;

        //* When
        let result: Result<(NpdmMetadata, AciData, AcidData), Error> = descriptor.try_into();

        //* Then
        let error = result.expect_err("priority above 63 should fail validation");
        assert!(
            matches!(
                error,
                Error::ValueOutOfRange {
                    field: "main_thread_priority",
                    value: 64,
                    max: 63
                }
            ),
            "Expected ValueOutOfRange for main_thread_priority, got {error:?}"
        );
    }

    #[test]
    fn try_from_with_conflicting_debug_flags_fails() {
        //* Given
        let descriptor = descriptor_with_capabilities(
            r#"[{ "type": "debug_flags", "value": { "allow_debug": true, "force_debug": true } }]"#,
        );

        //* When
        let result: Result<(NpdmMetadata, AciData, AcidData), Error> = descriptor.try_into();

        //* Then
        let error = result.expect_err("setting two debug flags should fail validation");
        assert!(
            matches!(error, Error::ConflictingDebugFlags),
            "Expected ConflictingDebugFlags, got {error:?}"
        );
    }

    #[cfg(feature = "json-schema")]
    #[test]
    fn schema_for_descriptor_produces_an_object_schema() {
        //* When
        let schema = schemars::schema_for!(NpdmDescriptor);

        //* Then
        let json = serde_json::to_value(&schema).expect("schema should serialize to JSON");
        assert_eq!(
            json.get("type").and_then(|value| value.as_str()),
            Some("object"),
            "the descriptor schema should describe an object, got {json}"
        );
    }
}
