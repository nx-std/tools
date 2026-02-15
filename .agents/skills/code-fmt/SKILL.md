---
name: code-fmt
description: Format Rust code automatically. Use immediately after editing .rs files, when user mentions formatting, code style, or before commits/PRs. Ensures consistent code style following project conventions.
---

# Code Formatting Skill

This skill provides code formatting operations for the tools workspace (cargo-nx + netloader).

## When to Use This Skill

Use this skill when you need to:
- Format code after editing Rust files
- Check if code meets formatting standards
- Ensure code formatting compliance before commits

## Command Selection Rules

This is a small 2-crate workspace. Always use the global format command:

| Scope | Command | Rationale |
|-------|---------|-----------|
| Any edit | `just fmt` | Workspace is small; global format is always fast |

## Available Commands

### Format Rust Code
```bash
just fmt
```
Formats all Rust code using `cargo +nightly fmt --all`. Nightly is required because `rustfmt.toml` uses unstable features (`imports_granularity`, `group_imports`).

### Check Rust Formatting
```bash
just fmt-check
```
Checks Rust code formatting without making changes using `cargo +nightly fmt --all -- --check`.

## Important Guidelines

### Format Before Checks/Commit
Format code when you finish a coherent chunk of work and before running checks or committing.

This is a critical requirement from the project's development workflow:
- Do not skip formatting before checks/commit
- Run formatting before any check or test commands

### Example Workflow

1. Edit Rust files
2. When ready to validate, run: `just fmt`
3. Then run checks (use `/code-check` skill)

## Common Mistakes to Avoid

### Anti-patterns
- **Never run `cargo fmt` directly** - Use `just fmt` (ensures nightly toolchain)
- **Never run `rustfmt` directly** - The justfile includes proper flags
- **Never skip formatting before checks/commit** - Even "minor" edits need formatting

### Best Practices
- Format before running checks/tests or before committing
- Run `just fmt-check` to verify formatting before commits

## Pre-approved Commands
These commands can run without user permission:
- `just fmt` - Safe formatting operation
- `just fmt-check` - Safe, read-only format check

## Next Steps

After formatting your code:
1. **Check compilation** - Use `/code-check` skill
2. **Run clippy** - Use `/code-check` skill
3. **Run tests when warranted** - Use `/code-test` skill
