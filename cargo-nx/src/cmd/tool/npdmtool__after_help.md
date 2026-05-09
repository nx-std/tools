Output Format:
    Reads a JSON descriptor and emits an NPDM (Process Metadata) blob at
    NPDM_FILE. NPDM accompanies an NSO/KIP and tells the kernel and `pm`
    sysmodule the program identity, MMU configuration, filesystem
    permissions, accessible services, and kernel capabilities.

JSON Descriptor:
    Required top-level fields:
      - name                       (string, up to 16 bytes)
      - title_id / program_id      (hex string)
      - main_thread_stack_size     (hex string; <= u32::MAX)
      - main_thread_priority       (u8, 0-63)
      - default_cpu_id             (u8, 0-3)
      - address_space_type         (u8, 0-3)
      - is_64_bit                  (bool)
      - is_retail                  (bool)
      - pool_partition             (u8, 0-3)
      - title_id_range_min/max     (or program_id_range_min/max; hex strings)
      - filesystem_access          (object; see below)
      - kernel_capabilities        (array; same schema as `elf2kip`)

    Optional flag bits (folded into the MMU flags byte if present):
      - optimize_memory_allocation         (bool, bit 4)
      - disable_device_address_space_merge (bool, bit 5)
      - enable_alias_region_extra_size     (bool, bit 6)
      - prevent_code_reads                 (bool, bit 7)

    Optional `version` / `process_category` (hex string, default 0).

Filesystem Access:
    The `filesystem_access` object configures FS permissions:
      - permissions          (hex string; bitmask of FsAccessFlag bits)
      - content_owner_ids    (array of hex strings; optional)
      - save_data_owner_ids  (array of { accessibility: u8 0-3, id: hex string };
                              optional)

Service Access:
    `service_host` lists services the process provides (hosted),
    `service_access` lists services it consumes as a client. Both arrays
    are optional; entries are service-name strings (max 8 bytes each).

Examples:
    # Convert a descriptor to NPDM
    cargo nx tool npdmtool main.json main.npdm
