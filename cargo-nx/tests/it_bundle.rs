//! End-to-end tests for `cargo nx bundle`.
//!
//! These tests drive the built `cargo-nx` binary the same way a user would,
//! packaging the `hello-world.elf` fixture as an NRO and validating the result
//! against the known-good `hello-world.nro` reference fixture.

use std::{
    path::Path,
    process::{Command, Output},
};

use tempfile::tempdir;

/// ELF fixture: a real Switch homebrew binary (graphics-printing example).
const ELF_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/hello-world.elf");

/// NRO reference: a known-good `hello-world.nro` built from `hello-world.elf`.
const NRO_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/hello-world.nro");

/// Offset of the `NRO0` header proper, past the 16-byte entry/`HOMEBREW` stub.
const NRO_HEADER_OFFSET: usize = 0x10;

/// Offset of the little-endian `u32` total-size field within the NRO header.
const NRO_SIZE_FIELD_OFFSET: usize = 0x18;

#[test]
fn bundle_with_no_nacp_produces_valid_nro() {
    //* Given
    let workdir = tempdir().expect("should create temp working directory");
    let output = workdir.path().join("hello-world.nro");
    let tmp_dir = workdir.path().join("bundle");

    //* When
    let result = run_bundle(Path::new(ELF_FIXTURE), &output, &tmp_dir, &["--no-nacp"]);

    //* Then
    assert!(
        result.status.success(),
        "bundle should exit successfully, stderr: {}",
        String::from_utf8_lossy(&result.stderr),
    );
    let nro = std::fs::read(&output).expect("bundle should write the NRO output file");
    assert_eq!(
        &nro[NRO_HEADER_OFFSET..NRO_HEADER_OFFSET + 4],
        b"NRO0",
        "output should carry the NRO0 header magic",
    );
    assert_eq!(
        nro_declared_size(&nro),
        nro.len(),
        "header size field should match the written file length",
    );
}

#[test]
fn bundle_with_no_nacp_matches_reference_nro() {
    //* Given
    let workdir = tempdir().expect("should create temp working directory");
    let output = workdir.path().join("hello-world.nro");
    let tmp_dir = workdir.path().join("bundle");
    let reference = std::fs::read(NRO_FIXTURE).expect("should read the NRO reference fixture");
    // The reference appends an ASET asset trailer; the NRO proper ends at the
    // header's declared size. Bytes 0x00..0x10 (the crt0 entry stub) come from
    // the same source ELF as the reference, so the entire NRO body must match.
    let expected = &reference[..nro_declared_size(&reference)];

    //* When
    let result = run_bundle(Path::new(ELF_FIXTURE), &output, &tmp_dir, &["--no-nacp"]);

    //* Then
    assert!(
        result.status.success(),
        "bundle should exit successfully, stderr: {}",
        String::from_utf8_lossy(&result.stderr),
    );
    let nro = std::fs::read(&output).expect("bundle should write the NRO output file");
    assert_eq!(
        &nro[..],
        expected,
        "bundled NRO must byte-match the reference NRO, entry stub included",
    );
}

#[test]
fn bundle_with_missing_nacp_field_fails() {
    //* Given
    let workdir = tempdir().expect("should create temp working directory");
    let output = workdir.path().join("hello-world.nro");
    let tmp_dir = workdir.path().join("bundle");

    //* When
    // NRO mode without `--no-nacp` requires --name/--author/--version.
    let result = run_bundle(Path::new(ELF_FIXTURE), &output, &tmp_dir, &[]);

    //* Then
    assert!(
        !result.status.success(),
        "bundle should fail when a required NACP field is missing",
    );
    assert!(
        !output.exists(),
        "bundle should not write an output file on failure",
    );
}

/// Run the `cargo-nx` binary's `bundle` subcommand in NRO mode.
fn run_bundle(input: &Path, output: &Path, tmp_dir: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-nx"))
        .args(["nx", "bundle"])
        .arg("--input")
        .arg(input)
        .arg("--output")
        .arg(output)
        .arg("--tmp-dir")
        .arg(tmp_dir)
        .args(extra)
        .output()
        .expect("cargo-nx binary should be executable")
}

/// Read the total NRO size declared in the header's size field.
fn nro_declared_size(nro: &[u8]) -> usize {
    let field: [u8; 4] = nro[NRO_SIZE_FIELD_OFFSET..NRO_SIZE_FIELD_OFFSET + 4]
        .try_into()
        .expect("NRO must contain a 4-byte size field");
    u32::from_le_bytes(field) as usize
}
