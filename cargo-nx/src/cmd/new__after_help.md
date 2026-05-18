Project Scaffolding:
    Proxies `cargo new` to create the package, then patches the result with
    the nx configuration: the `nx-std` dependency, a `[package.metadata.nx]`
    block, a `#![no_std]` crate root, and a `.cargo/config.toml` that enables
    `build-std` for the freestanding Switch target.

Package Types:
    --type selects what the project builds:
      - `nro`: a homebrew application packaged as an NRO (default)
      - `nsp`: an executable packaged as an NSP
      - `lib`: a library crate, with no nx packaging metadata

Delegation:
    Directory creation, package-name validation, and VCS setup are delegated
    to `cargo new`; --name, --edition, and --vcs are forwarded unchanged. The
    package name defaults to the destination directory name.

Examples:
    # New NRO application in ./my-app
    cargo nx new my-app

    # New NSP project with an explicit package name
    cargo nx new --type nsp --name my-sysmodule ./src/my-sysmodule

    # New library crate, without a version control system
    cargo nx new --type lib --vcs none ./my-lib
