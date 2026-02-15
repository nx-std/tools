# nx-tools

Tooling for Nintendo Switch Rust homebrew development.

## Tools

### cargo-nx

A cargo subcommand to simplify creating and building Nintendo Switch homebrew projects, avoiding the need for makefiles, scripts, or duplicated linker/target files across projects. Supports generating NRO and NSP formats after compilation.

#### Installation

1. If not already installed, install `rust-src`:

    ```bash
    rustup component add rust-src
    ```

2. Install `cargo-nx`:

    ```bash
    cargo install cargo-nx --git https://github.com/nx-std/tools
    ```

#### Usage

The program can be executed as `cargo-nx` or as a cargo subcommand (`cargo nx`), and provides the following subcommands:

**`new`** -- Create a new Rust project for the Nintendo Switch.

```
cargo nx new <path> [--name <name>] [--edition <2015|2018|2021>] [--type <lib|nro|nsp>]
```

**`build`** -- Build a Rust project for the Nintendo Switch.

```
cargo nx build [-r|--release] [-p <path>] [-t <triple>] [-v|--verbose]
```

Defaults to the `aarch64-nintendo-switch-freestanding` target triple.

**`link`** -- Send an NRO file to a Nintendo Switch running nx-hbmenu's netloader.

```
cargo nx link [options] <file.nro> [-- <nro args...>]
```

Options: `-a <ip>` (address), `-r <n>` (discovery retries), `-p <path>` (upload path), `-s` (start stdio server after transfer).

For detailed package format documentation (NRO/NACP fields, NSP/NPDM configuration), see [`cargo-nx/README.md`](cargo-nx/README.md).

### netloader

A Rust library implementing the nx-hbmenu netloader protocol for transferring NRO files to a Nintendo Switch over the network.

#### Protocol

The netloader protocol has three phases:

1. **Discovery** (UDP) -- The client broadcasts a `nxboot` ping to port 28280. The Switch responds with `bootnx` to port 28771, revealing its IP address.
2. **Transfer** (TCP) -- The client connects to port 28280, sends the file name and size, then streams the NRO data in zlib-compressed chunks. Command-line arguments for the NRO are sent after the file data.
3. **Stdio server** (TCP, optional) -- After transfer, the client can listen on port 28771 for stdout/stderr output redirected from the running NRO via libnx's nxlink stdio feature.

#### Usage

Add `netloader` as a dependency:

```toml
[dependencies]
netloader = { git = "https://github.com/nx-std/tools" }
```
