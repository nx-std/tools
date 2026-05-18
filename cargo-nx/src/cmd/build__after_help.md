Build Pipeline:
    Runs `cargo build` against the Nintendo Switch target triple, then reads
    each crate's `[package.metadata.nx]` table to decide how to package the
    compiled ELF.

Output Formats:
    The packaging step is selected by the metadata table:
      - `[package.metadata.nx.nro]`: the ELF is packaged as an NRO, with
        optional icon, NACP, and RomFS assets.
      - `[package.metadata.nx.nsp]`: the ELF is converted to an NSO and
        assembled with a compiled NPDM into a PFS0 (`.nsp`) archive.
    A crate with neither table is compiled but not packaged.

Target Selection:
    --target overrides the default `aarch64-nintendo-switch-freestanding`
    triple. The target spec is resolved from the RUST_TARGET_PATH environment
    variable, falling back to the workspace root.

Examples:
    # Build and package every nx crate in the workspace
    cargo nx build

    # Release build of a single package
    cargo nx build --release --package my-app

    # Forward feature flags to `cargo build`
    cargo nx build --features audio,debug
