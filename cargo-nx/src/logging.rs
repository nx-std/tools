//! The diagnostic logging channel.
//!
//! This is the operator's view, reached by setting `RUST_LOG`, and is separate from
//! what the user reads: everything the command means to tell the person who invoked
//! it goes through [`crate::ui`] instead.
//!
//! Errors are logged with two fields, `error` for the failure's own message and
//! `error_source` for the chain beneath it, so a filter keyed on either keeps
//! working as error types gain and lose layers.

/// The messages of `err`'s source chain, nearest cause first.
///
/// `err`'s own message is deliberately absent: it is logged separately as the
/// `error` field, and including it here would report a one-layer error as though it
/// had a cause.
///
/// Returns a `Debug` value rather than a `Vec`, so the chain renders as a list in
/// one field instead of one field per layer.
pub fn error_source(err: &dyn std::error::Error) -> tracing::field::DebugValue<Vec<String>> {
    let mut chain = Vec::new();
    let mut source = err.source();
    while let Some(cause) = source {
        chain.push(cause.to_string());
        source = cause.source();
    }
    tracing::field::debug(chain)
}

#[cfg(test)]
mod tests {
    use super::error_source;

    #[derive(Debug, thiserror::Error)]
    #[error("the root cause")]
    struct Root;

    #[derive(Debug, thiserror::Error)]
    #[error("the middle layer")]
    struct Middle(#[source] Root);

    #[derive(Debug, thiserror::Error)]
    #[error("the outermost failure")]
    struct Outer(#[source] Middle);

    #[test]
    fn error_source_lists_every_cause_beneath_the_error() {
        //* Given
        let err = Outer(Middle(Root));

        //* When
        let rendered = format!("{:?}", error_source(&err));

        //* Then
        assert_eq!(
            rendered, r#"["the middle layer", "the root cause"]"#,
            "the chain should reach the root cause, not stop at the immediate one"
        );
    }

    #[test]
    fn error_source_of_an_error_without_a_cause_is_empty() {
        //* Given
        let err = Root;

        //* When
        let rendered = format!("{:?}", error_source(&err));

        //* Then
        assert_eq!(
            rendered, "[]",
            "an error's own message belongs to the `error` field, not the chain"
        );
    }
}
