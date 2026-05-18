//! End-to-end tests for `cargo nx tool elf2nro`.
//!
//! These tests drive the built `cargo-nx` binary the same way a user would,
//! converting the `hello-world.elf` fixture and validating the resulting NRO
//! against the `hello-world.nro` reference produced by devkitPro's `elf2nro`.

use std::{path::Path, process::Command};

use tempfile::tempdir;

/// ELF fixture: a real Switch homebrew binary (graphics-printing example).
const ELF_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/hello-world.elf");

/// NRO reference: `hello-world.elf` converted by devkitPro's `elf2nro`.
const NRO_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/hello-world.nro");

/// Offset of the `NRO0` header proper, past the 16-byte entry/`HOMEBREW` stub.
const NRO_HEADER_OFFSET: usize = 0x10;

/// Offset of the little-endian `u32` total-size field within the NRO header.
const NRO_SIZE_FIELD_OFFSET: usize = 0x18;

/// Run the `cargo-nx` binary's `tool elf2nro` subcommand.
fn run_elf2nro(elf: &Path, nro: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-nx"))
        .args(["nx", "tool", "elf2nro"])
        .arg(elf)
        .arg(nro)
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

#[test]
fn elf2nro_with_elf_fixture_produces_valid_nro() {
    //* Given
    let workdir = tempdir().expect("should create temp working directory");
    let output = workdir.path().join("hello-world.nro");

    //* When
    let result = run_elf2nro(Path::new(ELF_FIXTURE), &output);

    //* Then
    assert!(
        result.status.success(),
        "elf2nro should exit successfully, stderr: {}",
        String::from_utf8_lossy(&result.stderr),
    );
    let nro = std::fs::read(&output).expect("elf2nro should write the NRO output file");
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
fn elf2nro_with_elf_fixture_matches_reference_nro() {
    //* Given
    let workdir = tempdir().expect("should create temp working directory");
    let output = workdir.path().join("hello-world.nro");
    let reference = std::fs::read(NRO_FIXTURE).expect("should read the NRO reference fixture");
    // The reference appends an ASET asset trailer; the NRO proper ends at the
    // header's declared size. The entry stub (bytes 0x00..0x10) is toolchain
    // specific, so the conversion is validated from the NRO0 header onward.
    let expected = &reference[NRO_HEADER_OFFSET..nro_declared_size(&reference)];

    //* When
    let result = run_elf2nro(Path::new(ELF_FIXTURE), &output);

    //* Then
    assert!(
        result.status.success(),
        "elf2nro should exit successfully, stderr: {}",
        String::from_utf8_lossy(&result.stderr),
    );
    let nro = std::fs::read(&output).expect("elf2nro should write the NRO output file");
    assert_eq!(
        &nro[NRO_HEADER_OFFSET..],
        expected,
        "converted NRO header and segments should match the devkitPro reference",
    );
}

#[test]
fn elf2nro_with_missing_elf_file_fails() {
    //* Given
    let workdir = tempdir().expect("should create temp working directory");
    let missing_elf = workdir.path().join("does-not-exist.elf");
    let output = workdir.path().join("out.nro");

    //* When
    let result = run_elf2nro(&missing_elf, &output);

    //* Then
    assert!(
        !result.status.success(),
        "elf2nro should fail when the input ELF is missing",
    );
    assert!(
        !output.exists(),
        "elf2nro should not write an output file on failure",
    );
}
