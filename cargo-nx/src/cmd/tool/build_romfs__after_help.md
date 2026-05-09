Output Format:
    Recursively walks IN_DIRECTORY and serializes its contents into a
    RomFS image at OUT_ROMFS_FILEPATH. RomFS is the read-only filesystem
    format used by Switch software for embedded assets — game data, fonts,
    shaders, configuration trees.

Directory Layout:
    The full directory tree is preserved. Files and subdirectories are
    laid out in the image exactly as they appear on disk relative to
    IN_DIRECTORY; symlinks are resolved against the source filesystem.
    The resulting image is suitable for embedding via `elf2nro --romfs`
    or for mounting at runtime.

Examples:
    # Build a RomFS image from an asset tree
    cargo nx tool build_romfs ./assets app.romfs

    # Combine with elf2nro to embed in an NRO
    cargo nx tool build_romfs ./assets app.romfs
    cargo nx tool elf2nro app.elf app.nro --romfs=app.romfs
