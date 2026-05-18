//! User-facing terminal output for `cargo-nx`.
//!
//! This module is the CLI's primary output channel: it renders the program's
//! product — progress, warnings, and errors — to stdout/stderr, mirroring
//! Cargo's output style. It is deliberately distinct from `tracing`, which
//! carries opt-in diagnostic logging (`RUST_LOG`); see `docs/code/logging.md`.

use std::{
    error::Error,
    io::{self, IsTerminal, Write},
};

use anstyle::{AnsiColor, Reset, Style};

/// Style of the `error:` prefix — bold red, matching Cargo.
const ERROR_STYLE: Style = AnsiColor::Red.on_default().bold();

/// Style of the `warning:` prefix — bold yellow, matching Cargo.
const WARNING_STYLE: Style = AnsiColor::Yellow.on_default().bold();

/// Style of a status verb — bold green, matching Cargo.
const STATUS_STYLE: Style = AnsiColor::Green.on_default().bold();

/// Width of the right-aligned status verb column, matching Cargo.
const STATUS_VERB_WIDTH: usize = 12;

/// Exit code for a generic command failure, matching Cargo's convention.
pub const EXIT_FAILURE: i32 = 101;

/// A command error that can be reported and mapped to a process exit code.
///
/// Every `cmd::*` error type implements this so `main` can report any command
/// failure uniformly. The default exit code is [`EXIT_FAILURE`]; commands that
/// wrap a child process override [`exit_code`] to propagate the child's code.
///
/// [`exit_code`]: CliError::exit_code
pub trait CliError: std::error::Error {
    /// The process exit code to use when this error aborts the program.
    fn exit_code(&self) -> i32 {
        EXIT_FAILURE
    }
}

/// Report `err` and its full source chain to stderr, Cargo style.
///
/// Prints an `error:`-prefixed line with `err`'s own message, followed — when
/// `err` has a source — by a `Caused by:` block listing every
/// [`Error::source`] in the chain.
pub fn error(err: &dyn Error) {
    let styled = io::stderr().is_terminal();
    let mut stderr = io::stderr().lock();
    // A failure to write the error report cannot itself be usefully reported.
    let _ = render_error(&mut stderr, err, styled);
}

/// Print a `warning:`-prefixed message to stderr.
pub fn warning(message: &str) {
    let styled = io::stderr().is_terminal();
    let mut stderr = io::stderr().lock();
    let _ = write_prefixed(&mut stderr, styled, WARNING_STYLE, "warning:", message);
}

/// Print a right-aligned, bold-green status verb followed by `message` to stdout.
///
/// Mirrors Cargo's progress lines, e.g. `    Finished` or `    Bundling`.
pub fn status(verb: &str, message: &str) {
    let styled = io::stdout().is_terminal();
    let mut stdout = io::stdout().lock();
    let _ = write_status(&mut stdout, styled, verb, message);
}

/// Forward already-rendered text (e.g. `cargo` / `rustc` diagnostics) verbatim to stdout.
pub fn raw(text: &str) {
    let mut stdout = io::stdout().lock();
    let _ = stdout.write_all(text.as_bytes());
}

/// Render `err` and its source chain into `w`.
///
/// Pure over its writer so it can be unit-tested without touching a terminal.
/// `styled` selects whether the `error:` prefix carries ANSI styling.
fn render_error(w: &mut impl Write, err: &dyn Error, styled: bool) -> io::Result<()> {
    write_prefix(w, styled, ERROR_STYLE, "error:")?;
    writeln!(w, " {err}")?;

    let mut source = err.source();
    if source.is_some() {
        writeln!(w, "\nCaused by:")?;
    }
    while let Some(cause) = source {
        for line in cause.to_string().lines() {
            writeln!(w, "  {line}")?;
        }
        source = cause.source();
    }

    Ok(())
}

/// Write a styled `label` (e.g. `error:`) to `w`, with no trailing space or newline.
fn write_prefix(w: &mut impl Write, styled: bool, style: Style, label: &str) -> io::Result<()> {
    if styled {
        write!(w, "{}{label}{}", style.render(), Reset.render())
    } else {
        write!(w, "{label}")
    }
}

/// Write a styled `label`, a space, `message`, and a newline to `w`.
fn write_prefixed(
    w: &mut impl Write,
    styled: bool,
    style: Style,
    label: &str,
    message: &str,
) -> io::Result<()> {
    write_prefix(w, styled, style, label)?;
    writeln!(w, " {message}")
}

/// Write a right-aligned status verb and `message` to `w`.
fn write_status(w: &mut impl Write, styled: bool, verb: &str, message: &str) -> io::Result<()> {
    let padding = STATUS_VERB_WIDTH.saturating_sub(verb.len());
    write!(w, "{:padding$}", "")?;
    write_prefix(w, styled, STATUS_STYLE, verb)?;
    writeln!(w, " {message}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("top-level failure")]
    struct TopError(#[source] MidError);

    #[derive(Debug, thiserror::Error)]
    #[error("middle failure")]
    struct MidError(#[source] LeafError);

    #[derive(Debug, thiserror::Error)]
    #[error("leaf failure")]
    struct LeafError;

    #[test]
    fn it_renders_error_with_full_cause_chain() {
        //* Given
        let err = TopError(MidError(LeafError));
        let mut buf = Vec::new();

        //* When
        render_error(&mut buf, &err, false).expect("rendering should succeed");

        //* Then
        let output = String::from_utf8(buf).expect("output should be valid UTF-8");
        assert_eq!(
            output,
            "error: top-level failure\n\nCaused by:\n  middle failure\n  leaf failure\n"
        );
    }

    #[test]
    fn it_renders_error_without_causes() {
        //* Given
        let err = LeafError;
        let mut buf = Vec::new();

        //* When
        render_error(&mut buf, &err, false).expect("rendering should succeed");

        //* Then
        let output = String::from_utf8(buf).expect("output should be valid UTF-8");
        assert_eq!(output, "error: leaf failure\n");
    }
}
