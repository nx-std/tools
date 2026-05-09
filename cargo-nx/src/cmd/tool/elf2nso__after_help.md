Output Format:
    Reads an ELF file produced by the Switch toolchain, parses its `.text`,
    `.rodata`, and `.data` segments, and emits an NSO (Nintendo Shared
    Object) container at NSO_FILE. The NSO is the format used by sysmodules
    and applets loaded by the Loader (`ldr`) system service.

Compression:
    The builder applies LZ4 compression to the code segments and records
    each segment's hash in the NSO header. No knobs are exposed — the
    output matches what the Loader expects for production sysmodules.

Examples:
    # Convert an ELF sysmodule to NSO
    cargo nx tool elf2nso sysmodule.elf sysmodule.nso
