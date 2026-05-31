# Display available commands (default target)
default:
    @just --list


## Format

alias fmt := fmt-rs
alias fmt-check := fmt-rs-check

# Format Rust code (cargo fmt --all)
[group: 'format']
fmt-rs:
    cargo +nightly fmt --all

# Check Rust code format (cargo fmt --check)
[group: 'format']
fmt-rs-check:
    cargo +nightly fmt --all -- --check


## Check

alias check := check-rs

# Check Rust code (cargo check --all-targets)
[group: 'check']
check-rs *EXTRA_FLAGS:
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

alias check-deps := check-unused-deps

# Check for unused Rust dependencies (cargo machete)
[group: 'check']
check-unused-deps:
    cargo machete


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


## Codegen

alias codegen := gen

GEN_SCHEMAS_OUTDIR := "docs/schemas"

# Generate the NPDM descriptor JSON schema
[group: 'codegen']
gen-npdm-schema:
    #!/usr/bin/env bash
    set -euo pipefail
    # Trigger the cargo-nx-gen build script with the schema-generation cfg flag.
    RUSTFLAGS="--cfg gen_schema_npdm" cargo check -p cargo-nx-gen
    mkdir -p {{GEN_SCHEMAS_OUTDIR}}
    cp -f $(ls -t target/debug/build/cargo-nx-gen-*/out/schema.json | head -1) {{GEN_SCHEMAS_OUTDIR}}/npdm.spec.json
    echo "  {{GEN_SCHEMAS_OUTDIR}}/npdm.spec.json"

# Run all codegen tasks
[group: 'codegen']
gen: gen-npdm-schema


## Clean

alias clean := cargo-clean

# Clean cargo workspace (cargo clean)
[group: 'clean']
cargo-clean:
    cargo clean


## Misc

PRECOMMIT_CONFIG := ".github/pre-commit-config.yaml"
PRECOMMIT_DEFAULT_HOOKS := "pre-commit pre-push"

# Install Git hooks
[group: 'misc']
install-git-hooks HOOKS=PRECOMMIT_DEFAULT_HOOKS:
    #!/usr/bin/env bash
    set -e # Exit on error

    # Check if pre-commit is installed
    if ! command -v "pre-commit" &> /dev/null; then
        >&2 echo "=============================================================="
        >&2 echo "Required command 'pre-commit' not available ❌"
        >&2 echo ""
        >&2 echo "Please install pre-commit using your preferred package manager"
        >&2 echo "  pip install pre-commit"
        >&2 echo "  pacman -S pre-commit"
        >&2 echo "  apt-get install pre-commit"
        >&2 echo "  brew install pre-commit"
        >&2 echo "=============================================================="
        exit 1
    fi

    # Install all Git hooks (see PRECOMMIT_DEFAULT_HOOKS for default hooks)
    pre-commit install --config {{PRECOMMIT_CONFIG}} {{replace_regex(HOOKS, "\\s*([a-z-]+)\\s*", "--hook-type $1 ")}}

# Remove Git hooks
[group: 'misc']
remove-git-hooks HOOKS=PRECOMMIT_DEFAULT_HOOKS:
    #!/usr/bin/env bash
    set -e # Exit on error

    # Check if pre-commit is installed
    if ! command -v "pre-commit" &> /dev/null; then
        >&2 echo "=============================================================="
        >&2 echo "Required command 'pre-commit' not available ❌"
        >&2 echo ""
        >&2 echo "Please install pre-commit using your preferred package manager"
        >&2 echo "  pip install pre-commit"
        >&2 echo "  pacman -S pre-commit"
        >&2 echo "  apt-get install pre-commit"
        >&2 echo "  brew install pre-commit"
        >&2 echo "=============================================================="
        exit 1
    fi

    # Remove all Git hooks (see PRECOMMIT_DEFAULT_HOOKS for default hooks)
    pre-commit uninstall --config {{PRECOMMIT_CONFIG}} {{replace_regex(HOOKS, "\\s*([a-z-]+)\\s*", "--hook-type $1 ")}}

# Install cargo-machete (unused dependency checker)
[group: 'misc']
install-cargo-machete:
    cargo install --locked cargo-machete

# Install cargo-nextest (faster test runner)
[group: 'misc']
install-cargo-nextest:
    cargo install --locked cargo-nextest
