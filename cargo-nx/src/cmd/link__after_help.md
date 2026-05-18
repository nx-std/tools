Netloader Deployment:
    Sends an NRO to a Nintendo Switch running nx-hbmenu's netloader — a Rust
    reimplementation of the `nxlink` tool. The console is located by UDP
    broadcast discovery unless --address pins it explicitly.

Upload Path:
    --path sets the destination on the console. A value ending in `.nro` is
    used verbatim; a value ending in `/` has the source file name appended.
    With no --path, the source file name is used.

Forwarding Arguments:
    Trailing ARGS, and any string passed via --args, are forwarded to the NRO
    as its command-line arguments. --server keeps a stdio server running after
    the transfer so the homebrew's stdout is streamed back to the terminal.

Examples:
    # Discover the console on the LAN and send an NRO
    cargo nx link app.nro

    # Send to a known console address
    cargo nx link --address 192.168.1.42 app.nro

    # Upload to an explicit destination path
    cargo nx link --path /switch/app/app.nro app.nro

    # Forward arguments to the homebrew and stream its stdout back
    cargo nx link --server --args "--level 3" app.nro
