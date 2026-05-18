---
name: code-test
description: Run cargo tests for the nx-std/tools workspace after format/check/clippy are green. Prefer the smallest relevant scope (per-crate). Use after behavior changes to validate, before commits/PRs, or when user mentions tests.
allowed-tools: "Bash(just test:*), Bash(just test-crate:*)"
---

# Code Testing Skill

Runs the cargo test suite for the `nx-std/tools` Rust workspace (`cargo-nx`, `nx-netloader`).

These are **host tests** — they compile and run on the development machine via `cargo test` (or `cargo nextest` if installed). There is no cross-compilation, no Switch hardware involved.

## Prerequisite

**`/code-format` and `/code-check` must be green first.** If dirty, return there — compile/clippy issues are faster to surface than a test run.

## Scope Selection

Prefer the smallest scope relevant to the change.

| Blast radius                                                       | Action                              |
|--------------------------------------------------------------------|-------------------------------------|
| None (docs/comments only)                                          | Skip; state why                     |
| Single-crate behavior change                                       | `just test-crate <CRATE>`           |
| Cross-crate change (workspace `Cargo.toml`, both crates edited)    | `just test`                         |
| Public API change in `nx-netloader` (consumed by `cargo-nx`)          | `just test`                         |

**Crate derivation.** Map edited file paths to crates: `cargo-nx/` → `cargo-nx`, `nx-netloader/` → `nx-netloader`.

## Available Commands

### Run Workspace Tests
```bash
just test [EXTRA_FLAGS]
```
Runs all tests in the workspace. Uses `cargo nextest run --workspace` when `cargo-nextest` is installed; otherwise falls back to `cargo test --workspace`.

### Run Tests for a Specific Crate
```bash
just test-crate <CRATE> [EXTRA_FLAGS]
```
Runs tests only for the specified crate. **Default command** unless a workspace signal fires.

Examples:
- `just test-crate cargo-nx`
- `just test-crate nx-netloader`
- `just test-crate cargo-nx -- some_test_name` — run a single test

## Workflow

1. Ensure `/code-format` and `/code-check` are green.
2. Pick scope — per-crate by default; workspace only on cross-crate signal.
3. Run the command.
4. On failure: read the test output, fix the regression, re-run. Do **not** mark the task complete until tests pass.
5. Report: which scope, pass/fail counts, and any tests intentionally skipped (with reason).

## Anti-patterns

- Running tests before `/code-check` is green.
- Running `cargo test` directly — use `just test` / `just test-crate` so cargo-nextest is preferred when available.
- Defaulting to the workspace command when only one crate changed.
- Marking a task complete with failing or unrun tests for changed behavior.

## Pre-approved Commands

These commands can run without user permission:
- `just test` — workspace tests, read-only.
- `just test-crate <crate>` — per-crate tests, read-only.

## Related Skills

- `/code-format` — Format before testing.
- `/code-check` — Must be green before running this skill.
- `/code-review` — Higher-level review against guidelines.
