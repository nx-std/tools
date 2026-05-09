//! `bin2s` subcommand — convert binary files into GAS assembly source.

use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::PathBuf,
};

use super::identifier::sanitize_identifier;

/// Entry point for the `bin2s` subcommand.
///
/// Reads each input file, emits one assembly block per non-empty file to the
/// configured output (stdout or `--output` file), and optionally writes a
/// matching C header when `--header` is provided.
pub fn handle_subcommand(args: Args) -> Result<(), Error> {
    let entries = read_entries(&args.inputs, args.apple_llvm)?;
    write_asm(&args, &entries)?;

    if let Some(header_path) = args.header.as_ref() {
        write_header(header_path, &entries)?;
    }

    Ok(())
}

#[derive(clap::Args)]
pub struct Args {
    /// Input binary files
    #[arg(required = true, num_args = 1..)]
    pub inputs: Vec<PathBuf>,

    /// Alignment passed to the .balign directive
    #[arg(short = 'a', long, default_value_t = 4)]
    pub alignment: u32,

    /// Emit Apple-LLVM-style assembler directives
    #[arg(long)]
    pub apple_llvm: bool,

    /// Also write a C header with extern declarations to PATH
    #[arg(short = 'H', long, value_name = "PATH")]
    pub header: Option<PathBuf>,

    /// Write asm output to PATH instead of stdout
    #[arg(short = 'o', long, value_name = "PATH")]
    pub output: Option<PathBuf>,
}

/// Errors from the `bin2s` subcommand.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to read an input file
    #[error("Failed to read input file '{}': {source}", path.display())]
    ReadInput { path: PathBuf, source: io::Error },

    /// Could not derive a valid identifier from the input filename
    #[error("Cannot derive a valid identifier from input filename '{}'", path.display())]
    InvalidFileName { path: PathBuf },

    /// Failed to write the assembly output to stdout
    #[error("Failed to write assembly output to stdout")]
    WriteStdout(#[source] io::Error),

    /// Failed to write the assembly output file
    #[error("Failed to write assembly output file '{}': {source}", path.display())]
    WriteOutputFile { path: PathBuf, source: io::Error },

    /// Failed to write the C header file
    #[error("Failed to write header file '{}': {source}", path.display())]
    WriteHeader { path: PathBuf, source: io::Error },
}

/// A single input file resolved to its sanitized identifiers and contents.
///
/// Holds two identifiers because the C tool deliberately uses different
/// sanitization rules on the asm and header sides under `--apple-llvm`.
struct Entry {
    asm_name: String,
    header_name: String,
    bytes: Vec<u8>,
}

/// Read every input path into an [`Entry`], skipping empty files with a warning.
///
/// Returns [`Error::InvalidFileName`] if a path lacks a usable basename or if
/// sanitization yields an empty identifier.
fn read_entries(inputs: &[PathBuf], apple_llvm: bool) -> Result<Vec<Entry>, Error> {
    let mut entries = Vec::with_capacity(inputs.len());

    for path in inputs {
        let basename = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::InvalidFileName { path: path.clone() })?;

        let asm_name = sanitize_identifier(basename, apple_llvm)
            .ok_or_else(|| Error::InvalidFileName { path: path.clone() })?;
        // The C tool always uses non-apple-llvm sanitization for headers,
        // even when the asm side prepends '_'. Replicate that asymmetry.
        let header_name = sanitize_identifier(basename, false)
            .ok_or_else(|| Error::InvalidFileName { path: path.clone() })?;

        let bytes = std::fs::read(path).map_err(|err| Error::ReadInput {
            path: path.clone(),
            source: err,
        })?;

        if bytes.is_empty() {
            tracing::warn!(path = %path.display(), "skipping empty input file");
            continue;
        }

        entries.push(Entry {
            asm_name,
            header_name,
            bytes,
        });
    }

    Ok(entries)
}

/// Write all entries to the asm output destination (stdout or file).
///
/// Errors are mapped to [`Error::WriteOutputFile`] or [`Error::WriteStdout`]
/// based on which destination was selected.
fn write_asm(args: &Args, entries: &[Entry]) -> Result<(), Error> {
    let has_header = args.header.is_some();

    match args.output.as_ref() {
        Some(path) => {
            let file = File::create(path).map_err(|err| Error::WriteOutputFile {
                path: path.clone(),
                source: err,
            })?;
            let mut writer = BufWriter::new(file);
            write_asm_to(&mut writer, args, entries, has_header)
                .and_then(|()| writer.flush())
                .map_err(|err| Error::WriteOutputFile {
                    path: path.clone(),
                    source: err,
                })
        }
        None => {
            let stdout = io::stdout();
            let mut writer = BufWriter::new(stdout.lock());
            write_asm_to(&mut writer, args, entries, has_header)
                .and_then(|()| writer.flush())
                .map_err(Error::WriteStdout)
        }
    }
}

/// Write every asm block to the given writer, in input order.
fn write_asm_to<W: Write>(
    writer: &mut W,
    args: &Args,
    entries: &[Entry],
    has_header: bool,
) -> io::Result<()> {
    for entry in entries {
        write_asm_block(writer, entry, args.alignment, args.apple_llvm, has_header)?;
    }
    Ok(())
}

/// Emit a single self-contained asm block for `entry`.
///
/// Block layout matches devkitPro's `bin2s` byte-for-byte: section/`.balign`
/// preamble, `.byte` rows, end label, optional `_size` constant (only when no
/// header is being generated), and the GNU-stack note trailer.
fn write_asm_block<W: Write>(
    writer: &mut W,
    entry: &Entry,
    alignment: u32,
    apple_llvm: bool,
    has_header: bool,
) -> io::Result<()> {
    writeln!(
        writer,
        "/* Generated by BIN2S - please don't edit directly */"
    )?;

    if apple_llvm {
        writer.write_all(b"\t.const_data\n")?;
    } else {
        writeln!(writer, "\t.section .rodata.{}, \"a\"", entry.asm_name)?;
    }

    writeln!(writer, "\t.balign {alignment}")?;
    writeln!(writer, "\t.global {}", entry.asm_name)?;
    writeln!(writer, "{}:", entry.asm_name)?;

    write_byte_lines_decimal(writer, &entry.bytes)?;

    write!(writer, "\n\n\t.global {}_end\n", entry.asm_name)?;
    write!(writer, "{}_end:\n\n", entry.asm_name)?;

    if !has_header {
        writeln!(writer, "\t.global {}_size", entry.asm_name)?;
        writer.write_all(b"\t.balign 4\n")?;
        writeln!(
            writer,
            "{}_size: .int {}",
            entry.asm_name,
            entry.bytes.len()
        )?;
    }

    write!(
        writer,
        "\n\n#if defined(__linux__) && defined(__ELF__)\n\
         .section .note.GNU-stack,\"\",%progbits\n\
         #endif"
    )?;

    Ok(())
}

/// Emit `bytes` as `.byte` rows of up to 16 right-aligned decimal values each.
///
/// Each new row is prefixed with `\t.byte ` and values are comma-separated
/// with no trailing comma, matching the C tool's `%3u` output format.
fn write_byte_lines_decimal<W: Write>(writer: &mut W, bytes: &[u8]) -> io::Result<()> {
    writer.write_all(b"\t.byte ")?;

    let last_index = bytes.len() - 1;
    let mut linelen = 0usize;

    for (index, byte) in bytes.iter().enumerate() {
        write!(writer, "{byte:3}")?;

        if index == last_index {
            break;
        }

        linelen += 1;
        if linelen >= 16 {
            linelen = 0;
            writer.write_all(b"\n\t.byte ")?;
        } else {
            writer.write_all(b",")?;
        }
    }

    Ok(())
}

/// Write the optional C header to `path`, mapping io errors to
/// [`Error::WriteHeader`].
fn write_header(path: &PathBuf, entries: &[Entry]) -> Result<(), Error> {
    let file = File::create(path).map_err(|err| Error::WriteHeader {
        path: path.clone(),
        source: err,
    })?;
    let mut writer = BufWriter::new(file);

    write_header_to(&mut writer, entries)
        .and_then(|()| writer.flush())
        .map_err(|err| Error::WriteHeader {
            path: path.clone(),
            source: err,
        })
}

/// Emit the header preamble and one extern/`_size` block per entry.
///
/// Uses the non-prefixed identifier form regardless of `--apple-llvm` so the
/// header remains includable from generic C/C++ translation units.
fn write_header_to<W: Write>(writer: &mut W, entries: &[Entry]) -> io::Result<()> {
    writer.write_all(
        b"/* Generated by BIN2S - please don't edit directly */\n\
          #pragma once\n\
          #include <stddef.h>\n\
          #include <stdint.h>\n\n",
    )?;

    for entry in entries {
        writeln!(writer, "extern const uint8_t {}[];", entry.header_name)?;
        writeln!(writer, "extern const uint8_t {}_end[];", entry.header_name)?;
        writer.write_all(b"#if __cplusplus >= 201103L\n")?;
        writeln!(
            writer,
            "static constexpr size_t {}_size={};",
            entry.header_name,
            entry.bytes.len()
        )?;
        writer.write_all(b"#else\n")?;
        writeln!(
            writer,
            "static const size_t {}_size={};",
            entry.header_name,
            entry.bytes.len()
        )?;
        writer.write_all(b"#endif\n")?;
    }

    Ok(())
}
