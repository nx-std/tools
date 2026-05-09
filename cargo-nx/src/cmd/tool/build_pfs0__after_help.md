Output Format:
    Walks IN_DIRECTORY, packs each regular file as an entry in a PFS0
    (Partition File System) archive, and writes the archive to
    OUT_PFS0_FILEPATH. PFS0 is the container used inside NSP packages and
    by `fs` for read-only partitions.

Directory Layout:
    Only the immediate contents of IN_DIRECTORY are packed. Subdirectories
    are not recursed — flatten or pre-arrange your inputs accordingly.
    Entry names in the archive are the file basenames as they appear on
    disk; the archive preserves no timestamps or permissions.

Examples:
    # Pack a directory into a PFS0 archive
    cargo nx tool build_pfs0 ./pfs0_root output.pfs0

    # Typical NSP staging: NCAs and ticket sit alongside one another
    cargo nx tool build_pfs0 ./nsp_staging title.nsp
