//! Identifier sanitization shared between the `bin2s` and `bin2c` subcommands.

/// Sanitize a filename basename into an identifier valid in both GAS and C.
///
/// The sanitization rules are:
/// - `-`, `.`, and `/` are replaced with `_`.
/// - Other non-alphanumeric, non-`_` characters are dropped.
/// - A leading `_` is prepended when the first surviving character is an
///   ASCII digit, or when `force_underscore_prefix` is set
///   (used by bin2s `--apple-llvm` mode).
///
/// Returns `None` when the result is empty (no characters survived sanitization).
pub fn sanitize_identifier(basename: &str, force_underscore_prefix: bool) -> Option<String> {
    let mut out = String::with_capacity(basename.len() + 1);

    for ch in basename.chars() {
        match ch {
            '-' | '.' | '/' => out.push('_'),
            c if c.is_ascii_alphanumeric() || c == '_' => out.push(c),
            _ => {}
        }
    }

    if out.is_empty() {
        return None;
    }

    let needs_prefix =
        force_underscore_prefix || out.chars().next().is_some_and(|c| c.is_ascii_digit());

    if needs_prefix {
        out.insert(0, '_');
    }

    Some(out)
}
