//! Static asset registry for the `new` subcommand.
//!
//! Single source of truth mapping each [`PackageKind`] to the embedded static
//! files used to scaffold a project on top of `cargo new`'s output.

use super::PackageKind;

/// Embed a template file from the crate's `assets/` directory.
///
/// `include_str!` requires a string literal, so the asset root cannot be a
/// `const`; this macro is the single place the directory path is spelled out.
macro_rules! include_template_str {
    ($path:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path))
    };
}

/// Contents of the generated `.cargo/config.toml`.
///
/// Enables `build-std` for the freestanding Switch target. Identical across
/// every package kind, so it is embedded once and shared.
pub const CARGO_CONFIG_TOML: &str = include_template_str!("cargo-config.toml.template");

/// Static crate-root source for a library package.
const LIB_RS: &str = include_template_str!("lib/lib.rs.template");

/// Static crate-root source for an NRO executable package.
const NRO_MAIN_RS: &str = include_template_str!("nro/main.rs.template");

/// Static crate-root source for an NSP executable package.
const NSP_MAIN_RS: &str = include_template_str!("nsp/main.rs.template");

/// `[package.metadata.nx.nro]` fragment merged into an NRO manifest.
const NRO_METADATA: &str = include_template_str!("nro/metadata.toml.template");

/// `[package.metadata.nx.nsp]` fragment merged into an NSP manifest.
const NSP_METADATA: &str = include_template_str!("nsp/metadata.toml.template");

/// The crate-root source file for a package kind.
///
/// Pairs the file name under `src/` with the static nx source that overwrites
/// the placeholder `cargo new` generates.
#[derive(Debug, Clone, Copy)]
pub struct CrateRoot {
    /// File name under the project's `src/` directory.
    pub file_name: &'static str,
    /// Static source that replaces the `cargo new`-generated placeholder.
    pub source: &'static str,
}

/// The crate-root source for `kind`.
///
/// A library uses `src/lib.rs`; an executable (NRO or NSP) uses `src/main.rs`,
/// matching the file `cargo new` creates for `--lib` and `--bin` respectively.
pub fn crate_root(kind: PackageKind) -> CrateRoot {
    match kind {
        PackageKind::Lib => CrateRoot {
            file_name: "lib.rs",
            source: LIB_RS,
        },
        PackageKind::Nro => CrateRoot {
            file_name: "main.rs",
            source: NRO_MAIN_RS,
        },
        PackageKind::Nsp => CrateRoot {
            file_name: "main.rs",
            source: NSP_MAIN_RS,
        },
    }
}

/// The static `[package.metadata.nx.*]` TOML fragment for `kind`.
///
/// Returns `None` for a library: libraries carry no nx packaging metadata.
/// The fragment's name field holds a placeholder that is overridden when the
/// manifest is patched (see [`super::manifest`]).
pub fn metadata_fragment(kind: PackageKind) -> Option<&'static str> {
    match kind {
        PackageKind::Lib => None,
        PackageKind::Nro => Some(NRO_METADATA),
        PackageKind::Nsp => Some(NSP_METADATA),
    }
}
