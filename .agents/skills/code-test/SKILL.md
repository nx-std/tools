---
name: code-test
description: Run tests to validate changes. Prefer the smallest relevant scope. Use after checks pass and when behavior changes warrant testing.
---

# Code Testing Skill

This skill provides testing operations for the tools workspace (cargo-nx + netloader). Uses cargo-nextest when available, with `cargo test` fallback.

## When to Use This Skill

Use this skill when you need to:
- Validate behavior changes or bug fixes
- Run tests for specific crates or the whole workspace
- Respond to a user request to run tests

## Test Scope Selection (Default: Minimal)

Start with the smallest scope that covers the change. Only broaden if you need more confidence.

- Docs/comments-only changes: skip tests and state why
- Localized code change in 1 crate: run `just test-crate <crate>`
- Cross-cutting changes affecting both crates: run `just test`

## Available Commands

### Run All Tests
```bash
just test [EXTRA_FLAGS]
```
Runs all tests in the workspace. Uses `cargo nextest run --workspace` if nextest is installed, falls back to `cargo test --workspace` with a warning.

Examples:
- `just test` - run all tests
- `just test -- --nocapture` - run with output visible

### Run Tests for Specific Crate
```bash
just test-crate <CRATE> [EXTRA_FLAGS]
```
Runs tests for a specific crate. Uses `cargo nextest run --package <CRATE>` if nextest is installed, falls back to `cargo test --package <CRATE>` with a warning.

Examples:
- `just test-crate cargo-nx` - run cargo-nx tests
- `just test-crate netloader` - run netloader tests
- `just test-crate netloader -- test_name` - run specific test

## Important Guidelines

### Test After Checks Pass
Only run tests after compilation and clippy checks pass. There is no point testing code that doesn't compile.

### All Tests Run Locally
Unlike projects with external dependencies, all tests in this workspace run locally with no special setup required.

### Example Workflow

**Single crate change:**
1. Edit files in `netloader`
2. Format: use `/code-fmt` skill
3. Check and lint: use `/code-check` skill
4. **Run tests**: `just test-crate netloader`
   - If failures: fix, return to step 2

**Cross-cutting change:**
1. Edit files in both crates
2. Format: use `/code-fmt` skill
3. Check and lint: use `/code-check` skill
4. **Run tests**: `just test`
   - If failures: fix, return to step 2

## Common Mistakes to Avoid

### Anti-patterns
- **Never run `cargo test` or `cargo nextest` directly** - Use justfile tasks
- **Never skip tests when behavior changes** - Only skip for docs/comments-only changes
- **Never ignore failing tests** - Fix them or document why they fail

### Best Practices
- Run the smallest relevant test scope
- Fix failing tests immediately
- Run tests for behavior changes or bug fixes

## Pre-approved Commands
These commands can run without user permission:
- `just test` - Safe, read-only test execution
- `just test-crate <crate>` - Safe, read-only test execution

## Validation Loop Pattern

```
Code Change -> Format -> Check -> Clippy -> Tests (when needed)
                 ^                             |
                 +---- Fix failures -----------+
```

If tests fail:
1. Read error messages carefully
2. Fix the issue
3. Format the fix (`just fmt`)
4. Check compilation (`just check-crate <crate>`)
5. Re-run the relevant tests (same scope as before)
6. Repeat until all pass

## Next Steps

After tests pass:
1. **Review changes** - Ensure quality before commits
2. **Commit** - All checks and tests must be green
