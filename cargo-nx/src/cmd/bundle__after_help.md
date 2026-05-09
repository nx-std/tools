Modes:
    NRO mode (default): packages the input ELF as an NRO with optional NACP,
    icon, and RomFS assets.
    NSP mode: selected automatically when `--npdm-json` is supplied. The ELF is
    converted to an NSO, the NPDM JSON is compiled, and both are assembled into
    a PFS0 (`.nsp`) archive in memory — no `exefs/` directory is written.

CLI Compatibility:
    Mirrors the legacy `mono/scripts/bundle.sh` flag set so existing build
    pipelines can swap the script for `cargo nx bundle` without changes.

Examples:
    # Bundle as NRO with NACP, icon, and RomFS
    cargo nx bundle \
        --input app.elf --output app.nro \
        --tmp-dir target/bundle \
        --name "App" --author "Me" --version "1.0.0" \
        --icon icon.jpg --romfs assets/romfs

    # Bundle as NSP from an NPDM JSON descriptor
    cargo nx bundle \
        --input app.elf --output app.nsp \
        --tmp-dir target/bundle \
        --npdm-json app.npdm.json
