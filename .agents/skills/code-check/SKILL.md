---
name: code-check
description: Validate and lint Rust code after changes. Use after editing .rs files, when user mentions compilation errors, type checking, linting, clippy warnings, or before commits/PRs. Ensures all code passes checks and has zero warnings.
---

# Code Checking Skill

This skill provides code validation and linting operations for the tools workspace (cargo-nx + netloader).

## When to Use This Skill

Use this skill when you need to:
- Validate Rust code after making changes
- Run compiler checks without building
- Lint code with clippy
- Ensure code quality before commits or PRs

## Command Selection Rules

Choose the appropriate command based on the number of crates edited:

| Crates Edited | Check Command | Clippy Command | Rationale |
|---------------|---------------|----------------|-----------|
| 1 crate | `just check-crate <crate>` | `just clippy-crate <crate>` | Faster, focused feedback |
| Both crates | `just check` | `just clippy` | More efficient than two per-crate calls |

**Decision process:**
1. Identify which crates contain your edited files
2. If only one crate: use per-crate commands
3. If both crates: use global commands

**Crate names in this workspace:**
- `cargo-nx` - Cargo subcommand for Switch homebrew
- `netloader` - Network deployment library

## Available Commands

### Check Rust Code
```bash
just check [EXTRA_FLAGS]
```
Checks Rust code using `cargo check --all-targets`. This runs the compiler without producing binaries.

Examples:
- `just check` - check all Rust code
- `just check --release` - check with release optimizations

### Check Specific Crate
```bash
just check-crate <CRATE> [EXTRA_FLAGS]
```
Checks a specific crate with all its targets using `cargo check --package <CRATE> --all-targets`.

Examples:
- `just check-crate cargo-nx` - check the cargo-nx crate
- `just check-crate netloader` - check the netloader crate

### Lint Rust Code (Clippy)
```bash
just clippy [EXTRA_FLAGS]
```
Lints all Rust code using `cargo clippy --all-targets`. Clippy catches common mistakes and suggests improvements.

Examples:
- `just clippy` - lint all code
- `just clippy -- -D warnings` - treat warnings as errors

### Lint Specific Crate
```bash
just clippy-crate <CRATE> [EXTRA_FLAGS]
```
Lints a specific crate using `cargo clippy --package <CRATE> --all-targets --no-deps`.

The `--no-deps` flag ensures clippy only analyzes the specified crate's code, not its dependencies. This provides:
- **Faster execution**: Skip checking dependency code you don't control
- **Focused output**: See only warnings from your crate, not transitive dependencies
- **Actionable results**: All warnings shown are in code you can fix

Examples:
- `just clippy-crate cargo-nx` - lint the cargo-nx crate only
- `just clippy-crate netloader` - lint the netloader crate only

## Important Guidelines

### MANDATORY: Run Checks After Changes
**You MUST run checks after making code changes. Use the Command Selection Rules above to choose the right command.**

Before considering a task complete: all checks MUST pass AND all clippy warnings MUST be fixed.

### Example Workflows

**Single crate edited:**
1. Edit files in `netloader` crate
2. Format: use `/code-fmt` skill
3. **Check compilation**: `just check-crate netloader`
   - If errors: fix, return to step 2
4. **Check clippy**: `just clippy-crate netloader`
   - If warnings: fix ALL, return to step 2
5. Repeat until: zero compilation errors AND zero clippy warnings

**Both crates edited:**
1. Edit files across both crates
2. Format: use `/code-fmt` skill
3. **Check compilation**: `just check`
   - If errors: fix, return to step 2
4. **Check clippy**: `just clippy`
   - If warnings: fix ALL, return to step 2
5. Repeat until: zero compilation errors AND zero clippy warnings

## Common Mistakes to Avoid

### Anti-patterns
- **Never run `cargo check` directly** - Use `just check-crate` or `just check`
- **Never run `cargo clippy` directly** - Justfile adds proper flags like `--no-deps`
- **Never ignore clippy warnings** - Fix all warnings before proceeding
- **Never skip the check step** - Even if "it should compile"

### Best Practices
- Follow Command Selection Rules based on number of crates edited
- Fix compilation errors before running clippy
- Run clippy when you finish a coherent chunk of work or before committing

## Pre-approved Commands
These commands can run without user permission:
- `just check` - Safe, read-only compilation check
- `just check-crate <crate>` - Safe, read-only compilation check
- `just clippy` - Safe, read-only linting
- `just clippy-crate <crate>` - Safe, read-only linting

## Next Steps

After all checks pass:
1. **Run tests when warranted** - Use `/code-test` skill
2. **Commit changes** - Ensure all checks green first
