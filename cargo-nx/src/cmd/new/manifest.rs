//! Manifest patching for the `new` subcommand.
//!
//! Pure, I/O-free transformations over a `cargo new`-generated `Cargo.toml`:
//! reading the package name back out, and patching in the `nx-std` dependency
//! and the `[package.metadata.nx.*]` block.

use toml_edit::{DocumentMut, InlineTable, Item, value};

use super::{PackageKind, assets};

/// Git source for the `nx-std` dependency added to every generated project.
const NX_STD_GIT: &str = "https://github.com/nx-std/mono";

/// Read `package.name` from a `cargo new`-generated manifest.
pub fn package_name(manifest: &str) -> Result<String, PackageNameError> {
    let doc: DocumentMut = manifest
        .parse()
        .map_err(|err| PackageNameError::Parse(Box::new(err)))?;
    doc.get("package")
        .and_then(Item::as_table)
        .and_then(|package| package.get("name"))
        .and_then(Item::as_str)
        .map(str::to_owned)
        .ok_or(PackageNameError::Missing)
}

/// Patch a `cargo new`-generated manifest with the nx-specific configuration.
///
/// Adds the `nx-std` dependency and, for an executable `kind`, merges the
/// `[package.metadata.nx.*]` block with its name field set to `name`. A library
/// `kind` receives only the dependency.
pub fn patch_manifest(
    manifest: &str,
    kind: PackageKind,
    name: &str,
) -> Result<String, PatchManifestError> {
    let mut doc: DocumentMut = manifest
        .parse()
        .map_err(|err| PatchManifestError::Parse(Box::new(err)))?;
    add_nx_std_dependency(&mut doc);
    if let Some(fragment) = assets::metadata_fragment(kind) {
        graft_metadata(&mut doc, fragment, kind, name).map_err(PatchManifestError::Metadata)?;
    }
    Ok(doc.to_string())
}

/// Insert `nx-std = { git = "..." }` into the manifest's `[dependencies]` table.
fn add_nx_std_dependency(doc: &mut DocumentMut) {
    let mut dependency = InlineTable::new();
    dependency.insert("git", NX_STD_GIT.into());
    doc["dependencies"]["nx-std"] = value(dependency);
}

/// Merge the static `[package.metadata.nx.*]` fragment into the manifest's
/// `[package]` table, then set the kind-specific name field to `name`.
fn graft_metadata(
    doc: &mut DocumentMut,
    fragment: &str,
    kind: PackageKind,
    name: &str,
) -> Result<(), MetadataError> {
    let parsed: DocumentMut = fragment
        .parse()
        .map_err(|err| MetadataError::Parse(Box::new(err)))?;
    let metadata = parsed
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(Item::as_table)
        .ok_or(MetadataError::FragmentShape)?
        .clone();

    doc["package"]["metadata"] = Item::Table(metadata);
    set_metadata_name(doc, kind, name);
    Ok(())
}

/// Override the kind-specific name field within the grafted nx metadata.
fn set_metadata_name(doc: &mut DocumentMut, kind: PackageKind, name: &str) {
    match kind {
        // A library carries no nx metadata, so there is no name field to set.
        PackageKind::Lib => {}
        PackageKind::Nro => {
            doc["package"]["metadata"]["nx"]["nro"]["nacp"]["default_name"] = value(name);
        }
        PackageKind::Nsp => {
            doc["package"]["metadata"]["nx"]["nsp"]["npdm"]["name"] = value(name);
        }
    }
}

/// Errors reading the package name from a generated manifest with
/// [`package_name`].
#[derive(Debug, thiserror::Error)]
pub enum PackageNameError {
    /// The manifest text is not valid TOML.
    #[error("manifest is not valid TOML")]
    Parse(#[source] Box<toml_edit::TomlError>),

    /// The manifest has no `package.name` string.
    ///
    /// `cargo new` always emits one, so this indicates the proxied `cargo new`
    /// produced an unexpected manifest.
    #[error("manifest has no `package.name`")]
    Missing,
}

/// Errors patching a generated manifest with [`patch_manifest`].
#[derive(Debug, thiserror::Error)]
pub enum PatchManifestError {
    /// The manifest text is not valid TOML.
    #[error("manifest is not valid TOML")]
    Parse(#[source] Box<toml_edit::TomlError>),

    /// The nx metadata fragment could not be merged into the manifest.
    #[error("failed to merge nx metadata")]
    Metadata(#[source] MetadataError),
}

/// Errors merging an embedded `[package.metadata.nx.*]` fragment.
#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    /// The embedded metadata fragment is not valid TOML.
    ///
    /// This indicates a malformed embedded asset rather than bad user input.
    #[error("embedded metadata fragment is not valid TOML")]
    Parse(#[source] Box<toml_edit::TomlError>),

    /// The embedded metadata fragment has no `package.metadata` table.
    #[error("embedded metadata fragment has no `package.metadata` table")]
    FragmentShape,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A manifest shaped like the output of `cargo new`.
    fn cargo_new_manifest() -> &'static str {
        r#"[package]
name = "demo"
version = "0.1.0"
edition = "2024"

[dependencies]
"#
    }

    #[test]
    fn package_name_extracts_name_from_manifest() {
        //* Given
        let manifest = cargo_new_manifest();

        //* When
        let result = package_name(manifest);

        //* Then
        let name = result.expect("the package name should be read");
        assert_eq!(name, "demo", "the name should match the manifest");
    }

    #[test]
    fn package_name_with_missing_package_table_fails() {
        //* Given
        let manifest = "[dependencies]\n";

        //* When
        let result = package_name(manifest);

        //* Then
        let error = result.expect_err("a manifest without `[package]` should fail");
        assert!(
            matches!(error, PackageNameError::Missing),
            "expected Missing, got {error:?}"
        );
    }

    #[test]
    fn patch_manifest_for_lib_adds_only_nx_std_dependency() {
        //* Given
        let manifest = cargo_new_manifest();

        //* When
        let patched = patch_manifest(manifest, PackageKind::Lib, "demo")
            .expect("patching a library manifest should succeed");

        //* Then
        assert!(
            patched.contains(r#"nx-std = { git = "https://github.com/nx-std/mono" }"#),
            "the manifest should declare the nx-std dependency"
        );
        assert!(
            !patched.contains("[package.metadata.nx"),
            "a library manifest should carry no nx metadata"
        );
    }

    #[test]
    fn patch_manifest_for_nro_adds_nacp_metadata() {
        //* Given
        let manifest = cargo_new_manifest();

        //* When
        let patched = patch_manifest(manifest, PackageKind::Nro, "demo")
            .expect("patching an nro manifest should succeed");

        //* Then
        assert!(
            patched.contains("[package.metadata.nx.nro.nacp]"),
            "the manifest should carry the nro nacp metadata"
        );
        assert!(
            patched.contains(r#"default_name = "demo""#),
            "the nacp default_name should match the package name"
        );
    }

    #[test]
    fn patch_manifest_for_nsp_adds_npdm_metadata() {
        //* Given
        let manifest = cargo_new_manifest();

        //* When
        let patched = patch_manifest(manifest, PackageKind::Nsp, "demo")
            .expect("patching an nsp manifest should succeed");

        //* Then
        assert!(
            patched.contains("[package.metadata.nx.nsp.npdm]"),
            "the manifest should carry the nsp npdm metadata"
        );
        assert!(
            patched.contains(r#"name = "demo""#),
            "the npdm name should match the package name"
        );
    }

    #[test]
    fn patch_manifest_overrides_metadata_name_with_package_name() {
        //* Given
        let manifest = cargo_new_manifest();

        //* When
        let patched = patch_manifest(manifest, PackageKind::Nro, "custom-name")
            .expect("patching with an explicit name should succeed");

        //* Then
        assert!(
            patched.contains(r#"default_name = "custom-name""#),
            "the name field should be overridden with the package name"
        );
        assert!(
            !patched.contains(r#"default_name = "unnamed""#),
            "the fragment placeholder should not survive patching"
        );
    }

    #[test]
    fn patch_manifest_output_is_valid_toml() {
        //* Given
        let manifest = cargo_new_manifest();

        //* When
        let patched = patch_manifest(manifest, PackageKind::Nsp, "demo")
            .expect("patching an nsp manifest should succeed");

        //* Then
        let reparsed = patched.parse::<DocumentMut>();
        assert!(
            reparsed.is_ok(),
            "the patched manifest should be valid TOML"
        );
    }

    #[test]
    fn patch_manifest_preserves_existing_dependencies() {
        //* Given
        let manifest = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
                         [dependencies]\nserde = \"1\"\n";

        //* When
        let patched =
            patch_manifest(manifest, PackageKind::Lib, "demo").expect("patching should succeed");

        //* Then
        assert!(
            patched.contains("serde = \"1\""),
            "the pre-existing dependency should be preserved"
        );
        assert!(
            patched.contains("nx-std"),
            "the nx-std dependency should be added alongside it"
        );
    }

    #[test]
    fn patch_manifest_with_malformed_toml_fails() {
        //* Given
        let manifest = "this is not = = valid toml";

        //* When
        let result = patch_manifest(manifest, PackageKind::Lib, "demo");

        //* Then
        let error = result.expect_err("malformed TOML should fail to patch");
        assert!(
            matches!(error, PatchManifestError::Parse(_)),
            "expected Parse, got {error:?}"
        );
    }

    #[test]
    fn nro_metadata_fragment_is_valid_toml() {
        //* Given
        let fragment = assets::metadata_fragment(PackageKind::Nro)
            .expect("an nro metadata fragment should exist");

        //* When
        let parsed = fragment.parse::<DocumentMut>();

        //* Then
        let doc = parsed.expect("the nro fragment should be valid TOML");
        assert!(
            doc.get("package")
                .and_then(|package| package.get("metadata"))
                .and_then(|metadata| metadata.get("nx"))
                .is_some(),
            "the fragment should expose `package.metadata.nx`"
        );
    }

    #[test]
    fn nsp_metadata_fragment_is_valid_toml() {
        //* Given
        let fragment = assets::metadata_fragment(PackageKind::Nsp)
            .expect("an nsp metadata fragment should exist");

        //* When
        let parsed = fragment.parse::<DocumentMut>();

        //* Then
        let doc = parsed.expect("the nsp fragment should be valid TOML");
        assert!(
            doc.get("package")
                .and_then(|package| package.get("metadata"))
                .and_then(|metadata| metadata.get("nx"))
                .is_some(),
            "the fragment should expose `package.metadata.nx`"
        );
    }
}
