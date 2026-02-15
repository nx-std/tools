default:
    @just --list

## Workspace management

alias clean := cargo-clean

# Clean cargo workspace
[group: 'workspace']
cargo-clean:
    cargo clean

## Format

# Format Rust code (cargo fmt --all)
[group: 'format']
fmt:
    cargo +nightly fmt --all

# Check Rust code format (cargo fmt --check)
[group: 'format']
fmt-check:
    cargo +nightly fmt --all -- --check

## Check

# Check Rust code (cargo check --all-targets)
[group: 'check']
check *EXTRA_FLAGS:
    cargo check --all-targets {{EXTRA_FLAGS}}

# Check specific crate (cargo check -p <crate> --all-targets)
[group: 'check']
check-crate CRATE *EXTRA_FLAGS:
    cargo check --package {{CRATE}} --all-targets {{EXTRA_FLAGS}}

# Lint Rust code (cargo clippy --all-targets)
[group: 'check']
clippy *EXTRA_FLAGS:
    cargo clippy --all-targets {{EXTRA_FLAGS}}

# Lint specific crate (cargo clippy -p <crate> --all-targets --no-deps)
[group: 'check']
clippy-crate CRATE *EXTRA_FLAGS:
    cargo clippy --package {{CRATE}} --all-targets --no-deps {{EXTRA_FLAGS}}

## Testing

# Run all tests in workspace
[group: 'test']
test *EXTRA_FLAGS:
    #!/usr/bin/env bash
    set -e
    if command -v "cargo-nextest" &> /dev/null; then
        cargo nextest run --workspace {{EXTRA_FLAGS}}
    else
        >&2 echo "================================================================="
        >&2 echo "WARNING: cargo-nextest not found - using 'cargo test' fallback"
        >&2 echo ""
        >&2 echo "For faster test execution, consider installing cargo-nextest:"
        >&2 echo "  cargo install --locked cargo-nextest@^0.9"
        >&2 echo "================================================================="
        sleep 1
        cargo test --workspace {{EXTRA_FLAGS}}
    fi

# Run tests for specific crate
[group: 'test']
test-crate CRATE *EXTRA_FLAGS:
    #!/usr/bin/env bash
    set -e
    if command -v "cargo-nextest" &> /dev/null; then
        cargo nextest run --package {{CRATE}} {{EXTRA_FLAGS}}
    else
        >&2 echo "================================================================="
        >&2 echo "WARNING: cargo-nextest not found - using 'cargo test' fallback"
        >&2 echo ""
        >&2 echo "For faster test execution, consider installing cargo-nextest:"
        >&2 echo "  cargo install --locked cargo-nextest@^0.9"
        >&2 echo "================================================================="
        sleep 1
        cargo test --package {{CRATE}} {{EXTRA_FLAGS}}
    fi
