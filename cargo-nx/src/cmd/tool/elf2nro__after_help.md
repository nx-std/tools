Output Format:
    Reads an ELF file produced by the Switch toolchain, parses its `.text`,
    `.rodata`, and `.data` segments, and emits an NRO (Nintendo Relocatable
    Object) container at NRO_FILE. The NRO carries the segments plus an
    optional Asset section (icon, NACP, RomFS) appended after the code image.

Asset Section:
    When any of --icon, --nacp, --romfs, or --romfsdir is provided, an
    Asset header is appended to the NRO so that homebrew loaders can find
    the icon, control data, and embedded RomFS image without a separate
    file. --romfs and --romfsdir are mutually exclusive.

RomFS Generation:
    --romfs accepts a pre-built RomFS image and embeds it verbatim.
    --romfsdir accepts a directory and serializes it into a RomFS image
    in-process before embedding (no intermediate file is written).

Aligned Header:
    --alignedheader sets the NRO `flags` field to 1, requesting the
    aligned-header layout used by some loaders. Omit unless your loader
    requires it; the default layout is correct for hbloader and nx-hbmenu.

Examples:
    # Minimal ELF -> NRO
    cargo nx tool elf2nro app.elf app.nro

    # Bundle icon and control data
    cargo nx tool elf2nro app.elf app.nro \
        --icon=icon.jpg --nacp=control.nacp

    # Embed a pre-built RomFS image
    cargo nx tool elf2nro app.elf app.nro --romfs=data.romfs

    # Build RomFS from a directory at packaging time
    cargo nx tool elf2nro app.elf app.nro --romfsdir=assets/

    # Request aligned-header layout
    cargo nx tool elf2nro app.elf app.nro --alignedheader
