Output Format:
    Reads an ELF file and a JSON descriptor, and emits a KIP1 (Kernel
    Initial Process) image at KIP_FILE. KIP1 is the format used by
    built-in processes loaded by the kernel at boot (e.g. `fs`, `loader`,
    `sm`, custom Atmosphère sysmodules).

JSON Descriptor:
    The descriptor configures the KIP1 header. Required top-level fields:
      - name                       (string, up to 12 bytes)
      - title_id / program_id      (u64; numeric or hex string "0x...")
      - main_thread_stack_size     (u64; numeric or hex string)
      - main_thread_priority       (u8)
      - default_cpu_id             (u8)
      - kernel_capabilities        (array; see below)

    Optional fields:
      - version / process_category (u32; default 1)
      - flags                      (u8; raw flag byte override)
      - use_secure_memory          (bool; default true; controls bit 5)
      - immortal                   (bool; default true; controls bit 6)

    When `flags` is present it is used verbatim. Otherwise flags start at
    0x3F and `use_secure_memory` / `immortal` toggle bits 5 and 6.

Kernel Capabilities:
    Each entry in the `kernel_capabilities` array has a `type` discriminant
    and a `value` payload. Supported types:
      - kernel_flags    { highest/lowest_thread_priority, highest/lowest_cpu_id }
      - syscalls        { name: id, ... } (id is u64 or hex string)
      - map             { address, size, is_ro, is_io }   (addresses in bytes;
                                                            converted to pages)
      - map_page        <address>                          (byte address; >> 12)
      - map_region      [{ region_type, is_ro }, ...]      (max 3 entries)
      - irq_pair        [irq0, irq1]                       (null = unused/0x3FF)
      - application_type        <u16>
      - min_kernel_version      <u64 or hex string>
      - handle_table_size       <u16>
      - debug_flags     { allow_debug, force_debug_prod, force_debug }

BSS Layout:
    If the ELF declares a non-empty BSS, its virtual address is placed
    immediately after the data segment, page-aligned to 0x1000 to prevent
    overlap with the kernel's page-granular mapping.

Examples:
    # Convert an ELF sysmodule to KIP1
    cargo nx tool elf2kip sysmodule.elf sysmodule.json sysmodule.kip
